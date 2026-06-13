use std::path::PathBuf;
use std::sync::Mutex;

use pnw_kernel::command::agent::AgentCommand;
use pnw_kernel::command::data::DataCommand;
use pnw_kernel::db::{crud, schema};
use pnw_kernel::handler::Handler;
use pnw_kernel::models;
use pnw_kernel::agent::llm::LlmEvent;
use tauri::{AppHandle, Emitter, State};

// ─── App State (复用连接和 runtime) ───

struct AppState {
    project_path: Mutex<Option<PathBuf>>,
    conn: Mutex<Option<rusqlite::Connection>>,
    rt: tokio::runtime::Runtime,
}

fn get_conn(state: &AppState) -> Result<std::sync::MutexGuard<'_, Option<rusqlite::Connection>>, String> {
    let mut guard = state.conn.lock().unwrap();
    if guard.is_some() {
        return Ok(guard);
    }
    let path = state.project_path.lock().unwrap().clone()
        .ok_or_else(|| "No project opened".to_string())?;
    let conn = rusqlite::Connection::open(path.join("project.db"))
        .map_err(|e| format!("DB error: {}", e))?;
    schema::init_schema(&conn).map_err(|e| format!("Schema error: {}", e))?;
    *guard = Some(conn);
    Ok(guard)
}

fn ensure_handler(state: &AppState) -> Result<Handler, String> {
    let project_path = state.project_path.lock().unwrap().clone()
        .ok_or_else(|| "No project opened".to_string())?;
    // 通过 get_conn 确保连接存在
    let guard = get_conn(state)?;
    // 把 conn 拿出来做个临时 handler（注意：这里克隆 connection 有性能问题，但仅用于非频繁操作）
    // 更好的做法是让 Handler 支持 &Connection
    drop(guard);
    // 重新打开（仅用于非频繁的 handler 操作）
    let conn = rusqlite::Connection::open(project_path.join("project.db"))
        .map_err(|e| format!("DB error: {}", e))?;
    schema::init_schema(&conn).map_err(|e| format!("Schema error: {}", e))?;
    Ok(Handler::new(conn, project_path))
}

// ─── Commands ───

#[tauri::command]
fn open_project(state: State<AppState>, path: String) -> Result<String, String> {
    let dir = PathBuf::from(&path);
    if !dir.join("project.db").exists() {
        return Err("No project.db found in directory".into());
    }
    *state.project_path.lock().unwrap() = Some(dir.clone());
    // 重置连接
    *state.conn.lock().unwrap() = None;
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
        id: id.clone(), name,
        created: now.clone(), modified: now,
        active: true, total_char: 0, chapter_char: 2000,
        sensitivity: models::Sensitivity::Normal,
    };
    crud::create_novel(&conn, &novel).map_err(|e| format!("Create novel error: {}", e))?;

    *state.project_path.lock().unwrap() = Some(dir.clone());
    *state.conn.lock().unwrap() = Some(conn);
    Ok(dir.display().to_string())
}

#[tauri::command]
fn get_project_path(state: State<AppState>) -> Result<Option<String>, String> {
    Ok(state.project_path.lock().unwrap().clone().map(|p| p.display().to_string()))
}

fn active_novel_id(conn: &rusqlite::Connection) -> Result<String, String> {
    let list = crud::list_novels(conn).map_err(|e| e.to_string())?;
    for n in &list { if n.active { return Ok(n.id.clone()); } }
    list.first().map(|n| n.id.clone()).ok_or_else(|| "No novels".to_string())
}

#[tauri::command]
fn get_outline(state: State<AppState>) -> Result<serde_json::Value, String> {
    let guard = get_conn(&state)?;
    let conn = guard.as_ref().ok_or("No connection")?;
    let novel_id = active_novel_id(conn)?;
    let cmd = DataCommand::GetOutlineTree { novel_id, phase_id: None };
    let handler = ensure_handler(&state)?;
    let output = handler.execute(cmd).map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(output).unwrap_or_default())
}

#[tauri::command]
fn get_chapter(state: State<AppState>, chapter_id: String) -> Result<serde_json::Value, String> {
    let guard = get_conn(&state)?;
    let conn = guard.as_ref().ok_or("No connection")?;
    let tc = crud::get_text_chapter(conn, &chapter_id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Chapter {} not found", chapter_id))?;

    let project_path = state.project_path.lock().unwrap().clone()
        .ok_or_else(|| "No project")?;
    let full_path = project_path.join(&tc.file_path);
    let content = pnw_kernel::storage::read_text(&full_path)
        .map_err(|e| format!("Read error: {}", e))?;

    Ok(serde_json::json!({
        "chapter": {
            "id": tc.id, "name": tc.name, "sort": tc.sort,
            "word_count": tc.word_count, "file_path": tc.file_path, "phase_id": tc.phase_id,
        },
        "content": content,
    }))
}

#[tauri::command]
fn save_chapter(state: State<AppState>, chapter_id: String, content: String) -> Result<(), String> {
    let guard = get_conn(&state)?;
    let conn = guard.as_ref().ok_or("No connection")?;
    let project_path = state.project_path.lock().unwrap().clone()
        .ok_or_else(|| "No project")?;

    let tc = crud::get_text_chapter(conn, &chapter_id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Chapter {} not found", chapter_id))?;

    let full_path = project_path.join(&tc.file_path);
    pnw_kernel::storage::write_text(&full_path, &content)
        .map_err(|e| format!("Write error: {}", e))?;

    let wc = pnw_kernel::storage::count_chars(&content);
    let mut updated = tc;
    updated.word_count = wc;
    crud::update_text_chapter(conn, &updated).map_err(|e| format!("DB error: {}", e))?;
    Ok(())
}

#[tauri::command]
fn agent_chat(
    app: AppHandle,
    state: State<'_, AppState>,
    command_type: String,
    chapter_id: Option<String>,
    message: String,
) -> Result<String, String> {
    let project_path = state.project_path.lock().unwrap().clone()
        .ok_or_else(|| "No project opened".to_string())?;

    // 打开 DB 连接
    let conn = rusqlite::Connection::open(project_path.join("project.db"))
        .map_err(|e| format!("DB error: {}", e))?;
    schema::init_schema(&conn).map_err(|e| format!("Schema error: {}", e))?;
    let novel_id = active_novel_id(&conn)?;

    // 确定命令类型
    let cmd = match command_type.as_str() {
        "evaluate" => {
            let cid = chapter_id.clone().or_else(|| {
                let phases = crud::list_text_phases(&conn, &novel_id).ok()?;
                let p = phases.last()?;
                let chs = crud::list_text_chapters(&conn, &p.id).ok()?;
                chs.last().map(|c| c.id.clone())
            }).ok_or_else(|| "No chapter to evaluate".to_string())?;
            AgentCommand::Evaluate { chapter_id: cid }
        }
        "revise" => {
            let cid = chapter_id.ok_or_else(|| "chapter_id required for revise".to_string())?;
            AgentCommand::ReviseChapter { chapter_id: cid, feedback: message }
        }
        "plan" => {
            AgentCommand::PlanOutline { novel_id, brief: message }
        }
        _ => { // "write" or default
            let cid = chapter_id.or_else(|| {
                let phases = crud::list_outline_phases(&conn, &novel_id).ok()?;
                phases.iter()
                    .filter_map(|p| crud::list_outline_chapters(&conn, &p.id).ok())
                    .flatten()
                    .find(|oc| oc.text_chapter_id.is_none())
                    .map(|oc| oc.id.clone())
            }).ok_or_else(|| "No chapter available".to_string())?;
            AgentCommand::WriteChapter { novel_id, chapter_id: cid, brief: message }
        }
    };

    // 创建事件通道，转发到前端
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<LlmEvent>();
    let app2 = app.clone();
    state.rt.spawn(async move {
        while let Some(event) = rx.recv().await {
            let payload = match event {
                LlmEvent::Token(t) => serde_json::json!({"type": "token", "data": t}),
                LlmEvent::Thinking(t) => serde_json::json!({"type": "thinking", "data": t}),
                LlmEvent::Step { name, status } => serde_json::json!({"type": "step", "name": name, "status": status}),
                LlmEvent::ToolCall { name, args } => serde_json::json!({"type": "tool_call", "name": name, "args": args}),
                LlmEvent::ToolResult { name, result } => serde_json::json!({"type": "tool_result", "name": name, "result": result}),
                LlmEvent::Done => serde_json::json!({"type": "done"}),
                LlmEvent::Error(e) => serde_json::json!({"type": "error", "data": e}),
            };
            let _ = app2.emit("llm-event", payload);
        }
    });

    let summary = state.rt.block_on(async {
        pnw_kernel::agent::execute_agent_command_stream(&conn, &project_path, cmd, tx).await
    }).map_err(|e| e.to_string())?;

    Ok(summary)
}

#[tauri::command]
fn get_stats(state: State<AppState>) -> Result<serde_json::Value, String> {
    let guard = get_conn(&state)?;
    let conn = guard.as_ref().ok_or("No connection")?;
    let novel_id = active_novel_id(conn)?;

    let novel = crud::get_novel(conn, &novel_id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| "Novel not found".to_string())?;

    let phases = crud::list_text_phases(conn, &novel_id).map_err(|e| e.to_string())?;
    let mut total_written = 0; let mut written_chapters = 0;
    for p in &phases {
        if let Ok(chs) = crud::list_text_chapters(conn, &p.id) {
            written_chapters += chs.len() as i32;
            for ch in &chs { total_written += ch.word_count; }
        }
    }

    let outline_phases = crud::list_outline_phases(conn, &novel_id).map_err(|e| e.to_string())?;
    let mut planned = 0;
    for p in &outline_phases {
        if let Ok(chs) = crud::list_outline_chapters(conn, &p.id) { planned += chs.len() as i32; }
    }

    Ok(serde_json::json!({
        "novel_name": novel.name, "total_char_target": novel.total_char,
        "chapter_char_target": novel.chapter_char, "total_written": total_written,
        "written_chapters": written_chapters, "planned_chapters": planned,
        "phases": phases.len(), "outline_phases": outline_phases.len(),
    }))
}

#[tauri::command]
fn list_characters(state: State<AppState>) -> Result<serde_json::Value, String> {
    let guard = get_conn(&state)?;
    let conn = guard.as_ref().ok_or("No connection")?;
    let novel_id = active_novel_id(conn)?;
    let handler = ensure_handler(&state)?;
    let output = handler.execute(DataCommand::ListCharacters { novel_id }).map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(output).unwrap_or_default())
}

#[tauri::command]
fn get_setting(state: State<AppState>) -> Result<serde_json::Value, String> {
    let guard = get_conn(&state)?;
    let conn = guard.as_ref().ok_or("No connection")?;
    let novel_id = active_novel_id(conn)?;
    let handler = ensure_handler(&state)?;
    let output = handler.execute(DataCommand::GetSetting { novel_id }).map_err(|e| e.to_string())?;
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
            conn: Mutex::new(None),
            rt: tokio::runtime::Runtime::new().expect("Failed to create tokio runtime"),
        })
        .invoke_handler(tauri::generate_handler![
            open_project, new_project, get_project_path,
            get_outline, get_chapter, save_chapter,
            agent_chat, get_stats, list_characters, get_setting,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
