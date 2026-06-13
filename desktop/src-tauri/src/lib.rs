use std::path::PathBuf;
use std::sync::Mutex;

use pnw_kernel::command::agent::AgentCommand;
use pnw_kernel::db::{crud, schema};
use pnw_kernel::handler::Handler;
use pnw_kernel::models;
use pnw_kernel::agent::llm::LlmEvent;
use tauri::{AppHandle, Emitter, State};

// ─── App State ───

struct AppState {
    project_path: Mutex<Option<PathBuf>>,
}

fn get_handler(state: &AppState) -> Result<Handler, String> {
    let path = state.project_path.lock().unwrap().clone()
        .ok_or_else(|| "No project opened".to_string())?;
    let conn = rusqlite::Connection::open(path.join("project.db"))
        .map_err(|e| format!("DB error: {}", e))?;
    schema::init_schema(&conn).map_err(|e| format!("Schema error: {}", e))?;
    Ok(Handler::new(conn, path))
}

// ─── Commands ───

#[tauri::command]
fn open_project(state: State<AppState>, path: String) -> Result<String, String> {
    let dir = PathBuf::from(&path);
    if !dir.join("project.db").exists() {
        return Err("No project.db found in directory".into());
    }
    *state.project_path.lock().unwrap() = Some(dir.clone());
    Ok(dir.canonicalize().unwrap_or(dir).display().to_string())
}

#[tauri::command]
fn new_project(state: State<AppState>, name: String) -> Result<String, String> {
    let dir = std::env::current_dir().unwrap_or_default().join(&name);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Create dir error: {}", e))?;
    std::fs::create_dir_all(dir.join("text")).ok();

    let conn = rusqlite::Connection::open(dir.join("project.db"))
        .map_err(|e| format!("DB error: {}", e))?;
    schema::init_schema(&conn).map_err(|e| format!("Schema error: {}", e))?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let novel = models::Novel {
        id: id.clone(),
        name,
        created: now.clone(),
        modified: now,
        active: true,
        total_char: 0,
        chapter_char: 2000,
        sensitivity: models::Sensitivity::Normal,
    };
    crud::create_novel(&conn, &novel).map_err(|e| format!("Create novel error: {}", e))?;
    *state.project_path.lock().unwrap() = Some(dir.clone());
    Ok(dir.display().to_string())
}

#[tauri::command]
fn get_project_path(state: State<AppState>) -> Result<Option<String>, String> {
    Ok(state.project_path.lock().unwrap().clone().map(|p| p.display().to_string()))
}

#[tauri::command]
fn get_outline(state: State<AppState>, novel_id: String) -> Result<serde_json::Value, String> {
    let mut handler = get_handler(&state)?;
    let cmd = pnw_kernel::command::data::DataCommand::GetOutlineTree {
        novel_id,
        phase_id: None,
    };
    let output = handler.execute(cmd).map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(output).unwrap_or_default())
}

#[tauri::command]
fn list_novels(state: State<AppState>) -> Result<serde_json::Value, String> {
    let mut handler = get_handler(&state)?;
    let output = handler.execute(pnw_kernel::command::data::DataCommand::ListNovels)
        .map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(output).unwrap_or_default())
}

#[tauri::command]
fn get_chapter(state: State<AppState>, chapter_id: String) -> Result<serde_json::Value, String> {
    let handler = get_handler(&state)?;

    // Read text chapter metadata
    let tc = crud::get_text_chapter(&handler.conn, &chapter_id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Chapter {} not found", chapter_id))?;

    let full_path = handler.project_path.join(&tc.file_path);
    let content = std::fs::read_to_string(&full_path)
        .map_err(|e| format!("Read error: {}", e))?;

    Ok(serde_json::json!({
        "chapter": {
            "id": tc.id,
            "name": tc.name,
            "sort": tc.sort,
            "word_count": tc.word_count,
            "file_path": tc.file_path,
            "phase_id": tc.phase_id,
        },
        "content": content,
    }))
}

#[tauri::command]
fn save_chapter(state: State<AppState>, chapter_id: String, content: String) -> Result<(), String> {
    let handler = get_handler(&state)?;
    let tc = crud::get_text_chapter(&handler.conn, &chapter_id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Chapter {} not found", chapter_id))?;

    let full_path = handler.project_path.join(&tc.file_path);
    std::fs::write(&full_path, &content).map_err(|e| format!("Write error: {}", e))?;

    let wc = content.chars().filter(|c| !c.is_whitespace()).count() as i32;
    let mut updated = tc;
    updated.word_count = wc;
    crud::update_text_chapter(&handler.conn, &updated).map_err(|e| format!("DB error: {}", e))?;
    Ok(())
}

#[tauri::command]
fn agent_chat(
    app: AppHandle,
    state: State<'_, AppState>,
    novel_id: String,
    message: String,
) -> Result<String, String> {
    let project_path = state.project_path.lock().unwrap().clone()
        .ok_or_else(|| "No project opened".to_string())?;

    let (_tx, rx) = std::sync::mpsc::channel::<LlmEvent>();
    let _app_clone = app.clone();

    // 转发事件到前端 (轮询 channel)
    let app_clone2 = app.clone();
    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            let payload = match event {
                LlmEvent::Token(t) => serde_json::json!({"type": "token", "data": t}),
                LlmEvent::Thinking(t) => serde_json::json!({"type": "thinking", "data": t}),
                LlmEvent::Step { name, status } => {
                    serde_json::json!({"type": "step", "name": name, "status": status})
                }
                LlmEvent::ToolCall { name, args } => {
                    serde_json::json!({"type": "tool_call", "name": name, "args": args})
                }
                LlmEvent::ToolResult { name, result } => {
                    serde_json::json!({"type": "tool_result", "name": name, "result": result})
                }
                LlmEvent::Done => serde_json::json!({"type": "done"}),
                LlmEvent::Error(e) => serde_json::json!({"type": "error", "data": e}),
            };
            app_clone2.emit("llm-event", payload).ok();
        }
    });

    let conn = rusqlite::Connection::open(project_path.join("project.db"))
        .map_err(|e| format!("DB error: {}", e))?;
    schema::init_schema(&conn).map_err(|e| format!("Schema error: {}", e))?;

    // 同步创建 tokio runtime 来运行 async agent
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let cmd = {
        let msg_lower = message.to_lowercase();
        if msg_lower.contains("评估") || msg_lower.contains("评价") {
            let phases = crud::list_text_phases(&conn, &novel_id).map_err(|e| e.to_string())?;
            let chapter_id = phases.last()
                .and_then(|p| crud::list_text_chapters(&conn, &p.id).ok())
                .and_then(|ch| ch.last().map(|c| c.id.clone()))
                .ok_or_else(|| "No chapters found".to_string())?;
            AgentCommand::Evaluate { chapter_id }
        } else if msg_lower.contains("大纲") && (msg_lower.contains("规划") || msg_lower.contains("生成")) {
            AgentCommand::PlanOutline { novel_id, brief: message }
        } else {
            let phases = crud::list_outline_phases(&conn, &novel_id).map_err(|e| e.to_string())?;
            let chapter_id = phases.iter()
                .filter_map(|p| crud::list_outline_chapters(&conn, &p.id).ok())
                .flatten()
                .find(|oc| oc.text_chapter_id.is_none())
                .map(|oc| oc.id.clone())
                .or_else(|| {
                    phases.last()
                        .and_then(|p| crud::list_text_chapters(&conn, &p.id).ok())
                        .and_then(|ch| ch.last().map(|c| c.id.clone()))
                })
                .ok_or_else(|| "No available chapters".to_string())?;
            AgentCommand::WriteChapter { novel_id, chapter_id, brief: message }
        }
    };

    // agent 执行需要 async + event channel
    let (async_tx, mut async_rx) = tokio::sync::mpsc::unbounded_channel::<LlmEvent>();
    let app_clone3 = app.clone();
    tokio::spawn(async move {
        while let Some(event) = async_rx.recv().await {
            let payload = match event {
                LlmEvent::Token(t) => serde_json::json!({"type": "token", "data": t}),
                LlmEvent::Thinking(t) => serde_json::json!({"type": "thinking", "data": t}),
                LlmEvent::Step { name, status } => {
                    serde_json::json!({"type": "step", "name": name, "status": status})
                }
                LlmEvent::ToolCall { name, args } => {
                    serde_json::json!({"type": "tool_call", "name": name, "args": args})
                }
                LlmEvent::ToolResult { name, result } => {
                    serde_json::json!({"type": "tool_result", "name": name, "result": result})
                }
                LlmEvent::Done => serde_json::json!({"type": "done"}),
                LlmEvent::Error(e) => serde_json::json!({"type": "error", "data": e}),
            };
            app_clone3.emit("llm-event", payload).ok();
        }
    });

    let summary = rt.block_on(async {
        pnw_kernel::agent::execute_agent_command_stream(
            &conn, &project_path, cmd, async_tx,
        ).await
    }).map_err(|e| e.to_string())?;

    Ok(summary)
}

#[tauri::command]
fn get_stats(state: State<AppState>, novel_id: String) -> Result<serde_json::Value, String> {
    let handler = get_handler(&state)?;
    let novel = crud::get_novel(&handler.conn, &novel_id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| "Novel not found".to_string())?;

    let phases = crud::list_text_phases(&handler.conn, &novel_id)
        .map_err(|e| format!("DB error: {}", e))?;
    let mut total_written = 0;
    let mut written_chapters = 0;
    for p in &phases {
        if let Ok(chs) = crud::list_text_chapters(&handler.conn, &p.id) {
            written_chapters += chs.len() as i32;
            for ch in &chs {
                total_written += ch.word_count;
            }
        }
    }

    let outline_phases = crud::list_outline_phases(&handler.conn, &novel_id)
        .map_err(|e| format!("DB error: {}", e))?;
    let mut planned_chapters = 0;
    for p in &outline_phases {
        if let Ok(chs) = crud::list_outline_chapters(&handler.conn, &p.id) {
            planned_chapters += chs.len() as i32;
        }
    }

    Ok(serde_json::json!({
        "novel_name": novel.name,
        "total_char_target": novel.total_char,
        "chapter_char_target": novel.chapter_char,
        "total_written": total_written,
        "written_chapters": written_chapters,
        "planned_chapters": planned_chapters,
        "phases": phases.len(),
        "outline_phases": outline_phases.len(),
    }))
}

#[tauri::command]
fn list_characters(state: State<AppState>, novel_id: String) -> Result<serde_json::Value, String> {
    let mut handler = get_handler(&state)?;
    let output = handler.execute(
        pnw_kernel::command::data::DataCommand::ListCharacters { novel_id }
    ).map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(output).unwrap_or_default())
}

#[tauri::command]
fn get_setting(state: State<AppState>, novel_id: String) -> Result<serde_json::Value, String> {
    let mut handler = get_handler(&state)?;
    let output = handler.execute(
        pnw_kernel::command::data::DataCommand::GetSetting { novel_id }
    ).map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(output).unwrap_or_default())
}

// ─── App Entry ───

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            project_path: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            open_project,
            new_project,
            get_project_path,
            get_outline,
            list_novels,
            get_chapter,
            save_chapter,
            agent_chat,
            get_stats,
            list_characters,
            get_setting,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
