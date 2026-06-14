use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, Query, State},
    http::Method,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use pnw_kernel::command::agent::AgentCommand;
use pnw_kernel::command::data::DataCommand;
use pnw_kernel::db::{crud, schema};
use pnw_kernel::handler::{Handler, Output};
use pnw_kernel::storage;
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

// ─── Shared state ───

struct AppState {
    project_path: PathBuf,
    /// Server start timestamp (Unix seconds), used as session identifier
    session_start: u64,
    /// Request counter for session tracking
    request_count: AtomicU64,
}

// ─── JSON envelope ───

#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    status: String,
    data: Option<T>,
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: T) -> Self {
        Self { status: "ok".into(), data: Some(data), error: None, error_code: None }
    }
}

impl ApiResponse<serde_json::Value> {
    fn from_output(output: Output) -> Self {
        Self {
            status: "ok".into(),
            data: Some(serde_json::to_value(output).unwrap_or_default()),
            error: None,
            error_code: None,
        }
    }
}

fn api_err(msg: impl ToString) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse {
        status: "error".into(),
        data: None,
        error: Some(msg.to_string()),
        error_code: None,
    })
}

fn api_err_with_code(msg: impl ToString, code: &str) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse {
        status: "error".into(),
        data: None,
        error: Some(msg.to_string()),
        error_code: Some(code.to_string()),
    })
}

fn api_ok(val: serde_json::Value) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(val))
}

// ─── Helpers ───

fn open_fresh_handler(state: &AppState) -> Result<Handler, String> {
    Ok(Handler::new(
        rusqlite::Connection::open(state.project_path.join("project.db"))
            .map_err(|e| format!("DB error: {}", e))?,
        state.project_path.clone(),
    ))
}

fn active_novel_id(conn: &rusqlite::Connection) -> Result<String, String> {
    let list = crud::list_novels(conn).map_err(|e| e.to_string())?;
    list.first()
        .map(|n| n.id.clone())
        .ok_or_else(|| "No novels found".into())
}

// ─── Routes ───

pub async fn run_server(host: &str, port: u16, project: Option<&str>, cors_origin: &str) {
    let project_path = if let Some(p) = project {
        PathBuf::from(p)
    } else if let Ok(p) = std::env::var("PNW_PROJECT") {
        PathBuf::from(p)
    } else {
        std::env::current_dir().expect("Cannot get current dir")
    };

    if !project_path.join("project.db").exists() {
        eprintln!("Error: no project.db found in {}", project_path.display());
        eprintln!("Create one with: pnw novel new <name>");
        std::process::exit(1);
    }

    // Verify DB can be opened
    let conn = rusqlite::Connection::open(project_path.join("project.db"))
        .expect("Cannot open database");
    schema::init_schema(&conn).expect("Cannot init schema");
    drop(conn);

    // Clean up orphan .tmp files from interrupted atomic writes
    if let Ok(n) = storage::cleanup_orphan_tmp(&project_path) {
        if n > 0 {
            eprintln!("Cleaned up {} orphan .tmp files", n);
        }
    }

    let session_start = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let state = Arc::new(AppState {
        project_path,
        session_start,
        request_count: AtomicU64::new(0),
    });

    let app = Router::new()
        .route("/", get(gateway_index))
        .route("/gateway/style.css", get(gateway_css))
        .route("/gateway/app.js", get(gateway_js))
        .route("/api/status", get(api_status))
        .route("/api/health", get(api_health))
        .route("/api/tools", get(api_tools))
        .route("/api/project", get(api_project))
        .route("/api/outline", get(api_outline))
        .route("/api/chapters", get(api_chapters))
        .route("/api/chapter/{id}", get(api_chapter_get).put(api_chapter_save))
        .route("/api/characters", get(api_characters_list).post(api_character_create))
        .route("/api/setting", get(api_setting_get).post(api_setting_update))
        .route("/api/samples", get(api_samples_list))
        .route("/api/stats", get(api_stats))
        .route("/api/agent/write", post(api_agent_write))
        .route("/api/agent/revise", post(api_agent_revise))
        .route("/api/agent/evaluate/{id}", post(api_agent_evaluate))
        .route("/api/export/txt", post(api_export_txt))
        .route("/api/command", post(api_command))
        .layer(if cors_origin == "*" {
            CorsLayer::permissive()
        } else {
            let origin: axum::http::HeaderValue = cors_origin.parse().unwrap();
            CorsLayer::new()
                .allow_origin(origin)
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers(Any)
        })
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    eprintln!("PNW Server starting on http://{}", addr);
    eprintln!("Gateway UI: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ─── Gateway UI ───

async fn gateway_index() -> Html<&'static str> { Html(GATEWAY_HTML) }
async fn gateway_css() -> impl IntoResponse {
    ([(axum::http::header::CONTENT_TYPE, "text/css; charset=utf-8")], GATEWAY_CSS)
}
async fn gateway_js() -> impl IntoResponse {
    ([(axum::http::header::CONTENT_TYPE, "application/javascript; charset=utf-8")], GATEWAY_JS)
}

// ─── API handlers ───

async fn api_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let count = state.request_count.fetch_add(1, Ordering::Relaxed) + 1;
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "name": "PrivateNovelWriter",
        "session_start": state.session_start,
        "request_count": count,
    }))
}

/// Health check — lightweight, no DB needed
async fn api_health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status":"ok","service":"pnw-server"}))
}

/// Tool discovery — returns available commands and endpoints
async fn api_tools() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "endpoints": [
            {"path":"/api/health","method":"GET","description":"健康检查"},
            {"path":"/api/status","method":"GET","description":"Server 信息（版本+session）"},
            {"path":"/api/project","method":"GET","description":"项目基本信息"},
            {"path":"/api/outline","method":"GET","description":"大纲树"},
            {"path":"/api/chapters","method":"GET","description":"所有正文章节列表"},
            {"path":"/api/chapter/{id}","method":"GET","description":"读取章节正文"},
            {"path":"/api/chapter/{id}","method":"PUT","description":"保存章节正文"},
            {"path":"/api/characters","method":"GET","description":"角色列表"},
            {"path":"/api/characters","method":"POST","description":"创建角色"},
            {"path":"/api/setting","method":"GET","description":"读取世界观设定"},
            {"path":"/api/setting","method":"POST","description":"更新世界观设定"},
            {"path":"/api/samples","method":"GET","description":"文风样例列表"},
            {"path":"/api/stats","method":"GET","description":"项目统计"},
            {"path":"/api/export/txt","method":"GET","description":"导出合并全文"},
            {"path":"/api/agent/write","method":"POST","description":"写正文（Agent B）"},
            {"path":"/api/agent/revise","method":"POST","description":"修改正文（Agent B）"},
            {"path":"/api/agent/evaluate/{id}","method":"POST","description":"评估章节（Agent B）"},
            {"path":"/api/command","method":"POST","description":"通用命令接口"},
            {"path":"/api/tools","method":"GET","description":"工具发现（本端点）"}
        ],
        "commands": [
            "get_outline","get_novel","list_characters","get_setting",
            "list_samples","get_plugin","list_outline_phases","list_outline_chapters",
            "create_outline_phase","create_outline_chapter","create_character","write_setting"
        ]
    }))
}

async fn api_project(State(state): State<Arc<AppState>>) -> Json<ApiResponse<serde_json::Value>> {
    let handler = match open_fresh_handler(&state) {
        Ok(h) => h,
        Err(e) => return api_err(e),
    };
    let novel_id = match active_novel_id(&handler.conn) {
        Ok(id) => id,
        Err(e) => return api_err(e),
    };
    match crud::get_novel(&handler.conn, &novel_id) {
        Ok(Some(n)) => api_ok(serde_json::json!({
            "id": n.id, "name": n.name, "total_char": n.total_char,
            "chapter_char": n.chapter_char, "created": n.created, "modified": n.modified,
        })),
        _ => api_err("Novel not found"),
    }
}

async fn api_outline(State(state): State<Arc<AppState>>) -> Json<ApiResponse<serde_json::Value>> {
    let handler = match open_fresh_handler(&state) {
        Ok(h) => h,
        Err(e) => return api_err(e),
    };
    let novel_id = match active_novel_id(&handler.conn) {
        Ok(id) => id,
        Err(e) => return api_err(e),
    };
    match handler.execute(DataCommand::GetOutlineTree { novel_id, phase_id: None }) {
        Ok(output) => Json(ApiResponse::from_output(output)),
        Err(e) => api_err(e.to_string()),
    }
}

async fn api_chapters(State(state): State<Arc<AppState>>) -> Json<ApiResponse<serde_json::Value>> {
    let handler = match open_fresh_handler(&state) {
        Ok(h) => h,
        Err(e) => return api_err(e),
    };
    let novel_id = match active_novel_id(&handler.conn) {
        Ok(id) => id,
        Err(e) => return api_err(e),
    };
    let phases = match crud::list_text_phases(&handler.conn, &novel_id) {
        Ok(p) => p,
        Err(e) => return api_err(e.to_string()),
    };
    let mut all = Vec::new();
    for phase in &phases {
        if let Ok(chs) = crud::list_text_chapters(&handler.conn, &phase.id) {
            for ch in chs {
                all.push(serde_json::json!({
                    "id": ch.id, "phase_id": ch.phase_id, "phase_name": phase.name,
                    "name": ch.name, "sort": ch.sort, "word_count": ch.word_count,
                }));
            }
        }
    }
    api_ok(serde_json::json!(all))
}

async fn api_chapter_get(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    let handler = match open_fresh_handler(&state) {
        Ok(h) => h,
        Err(e) => return api_err(e),
    };
    let tc = match crud::get_text_chapter(&handler.conn, &id) {
        Ok(Some(c)) => c,
        Ok(None) => return api_err("Chapter not found"),
        Err(e) => return api_err(e.to_string()),
    };
    let full_path = state.project_path.join(&tc.file_path);
    let content = storage::read_text(&full_path).unwrap_or_default();
    api_ok(serde_json::json!({
        "id": tc.id, "name": tc.name, "sort": tc.sort, "word_count": tc.word_count,
        "phase_id": tc.phase_id, "content": content, "file_path": tc.file_path,
    }))
}

#[derive(Deserialize)]
struct SaveBody { content: String }

async fn api_chapter_save(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
    Json(body): Json<SaveBody>,
) -> Json<ApiResponse<&'static str>> {
    let handler = match open_fresh_handler(&state) {
        Ok(h) => h,
        Err(e) => return Json(ApiResponse::err_inner(e)),
    };
    let tc = match crud::get_text_chapter(&handler.conn, &id) {
        Ok(Some(c)) => c,
        Ok(None) => return Json(ApiResponse::err_inner("Chapter not found")),
        Err(e) => return Json(ApiResponse::err_inner(e.to_string())),
    };
    let full_path = state.project_path.join(&tc.file_path);
    if let Err(e) = storage::write_text(&full_path, &body.content) {
        return Json(ApiResponse::err_inner(format!("Write error: {}", e)));
    }
    let wc = storage::count_chars(&body.content);
    let mut updated = tc;
    updated.word_count = wc;
    crud::update_text_chapter(&handler.conn, &updated).ok();
    Json(ApiResponse { status: "ok".into(), data: Some("saved"), error: None, error_code: None })
}

impl ApiResponse<&'static str> {
    fn err_inner(msg: impl ToString) -> Self {
        Self { status: "error".into(), data: None, error: Some(msg.to_string()), error_code: None }
    }
    fn err_code(msg: impl ToString, code: &str) -> Self {
        Self { status: "error".into(), data: None, error: Some(msg.to_string()), error_code: Some(code.to_string()) }
    }
}

async fn api_characters_list(State(state): State<Arc<AppState>>) -> Json<ApiResponse<serde_json::Value>> {
    let handler = match open_fresh_handler(&state) {
        Ok(h) => h,
        Err(e) => return api_err(e),
    };
    let novel_id = match active_novel_id(&handler.conn) {
        Ok(id) => id,
        Err(e) => return api_err(e),
    };
    match handler.execute(DataCommand::ListCharacters { novel_id }) {
        Ok(output) => Json(ApiResponse::from_output(output)),
        Err(e) => api_err(e.to_string()),
    }
}

#[derive(Deserialize)]
struct CreateCharBody {
    name: String,
    #[serde(default = "two")]
    char_type: i32,
    #[serde(default)]
    age: i32,
    #[serde(default)]
    relationship: String,
}
fn two() -> i32 { 2 }

async fn api_character_create(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateCharBody>,
) -> Json<ApiResponse<serde_json::Value>> {
    let handler = match open_fresh_handler(&state) {
        Ok(h) => h,
        Err(e) => return api_err(e),
    };
    let novel_id = match active_novel_id(&handler.conn) {
        Ok(id) => id,
        Err(e) => return api_err(e),
    };
    let id = uuid::Uuid::new_v4().to_string();
    match handler.execute(DataCommand::CreateCharacter {
        id, novel_id, name: body.name, char_type: body.char_type,
        age: body.age, relationship: body.relationship,
    }) {
        Ok(output) => Json(ApiResponse::from_output(output)),
        Err(e) => api_err(e.to_string()),
    }
}

async fn api_setting_get(State(state): State<Arc<AppState>>) -> Json<ApiResponse<serde_json::Value>> {
    let handler = match open_fresh_handler(&state) {
        Ok(h) => h,
        Err(e) => return api_err(e),
    };
    let novel_id = match active_novel_id(&handler.conn) {
        Ok(id) => id,
        Err(e) => return api_err(e),
    };
    match handler.execute(DataCommand::GetSetting { novel_id }) {
        Ok(output) => Json(ApiResponse::from_output(output)),
        Err(e) => api_err(e.to_string()),
    }
}

#[derive(Deserialize)]
struct UpdateSettingBody {
    title: Option<String>,
    inspiration: Option<String>,
    description: Option<String>,
    novel_type: Option<i32>,
    tags: Option<Vec<String>>,
}

async fn api_setting_update(
    State(state): State<Arc<AppState>>,
    Json(body): Json<UpdateSettingBody>,
) -> Json<ApiResponse<serde_json::Value>> {
    let handler = match open_fresh_handler(&state) {
        Ok(h) => h,
        Err(e) => return api_err(e),
    };
    let novel_id = match active_novel_id(&handler.conn) {
        Ok(id) => id,
        Err(e) => return api_err(e),
    };
    let existing = crud::get_setting(&handler.conn, &novel_id).ok().flatten();
    let cmd = DataCommand::WriteSetting {
        novel_id: novel_id.clone(),
        title: body.title.unwrap_or_else(|| existing.as_ref().map_or(String::new(), |s| s.title.clone())),
        inspiration: body.inspiration.unwrap_or_else(|| existing.as_ref().map_or(String::new(), |s| s.inspiration.clone())),
        description: body.description.unwrap_or_else(|| existing.as_ref().map_or(String::new(), |s| s.description.clone())),
        novel_type: body.novel_type.unwrap_or_else(|| existing.as_ref().map_or(0, |s| s.novel_type.to_i32())),
        tags: body.tags.unwrap_or_else(|| existing.as_ref().map_or(vec![], |s| s.tags.clone())),
    };
    match handler.execute(cmd) {
        Ok(output) => Json(ApiResponse::from_output(output)),
        Err(e) => api_err(e.to_string()),
    }
}

async fn api_samples_list(State(state): State<Arc<AppState>>) -> Json<ApiResponse<serde_json::Value>> {
    let handler = match open_fresh_handler(&state) {
        Ok(h) => h,
        Err(e) => return api_err(e),
    };
    let novel_id = match active_novel_id(&handler.conn) {
        Ok(id) => id,
        Err(e) => return api_err(e),
    };
    match handler.execute(DataCommand::ListSamples { novel_id }) {
        Ok(output) => Json(ApiResponse::from_output(output)),
        Err(e) => api_err(e.to_string()),
    }
}

async fn api_stats(State(state): State<Arc<AppState>>) -> Json<ApiResponse<serde_json::Value>> {
    let handler = match open_fresh_handler(&state) {
        Ok(h) => h,
        Err(e) => return api_err(e),
    };
    let novel_id = match active_novel_id(&handler.conn) {
        Ok(id) => id,
        Err(e) => return api_err(e),
    };
    let novel = match crud::get_novel(&handler.conn, &novel_id) {
        Ok(Some(n)) => n,
        _ => return api_err("Novel not found"),
    };
    let phases = crud::list_text_phases(&handler.conn, &novel_id).unwrap_or_default();
    let mut total_written = 0i32;
    let mut written_chapters = 0usize;
    for p in &phases {
        if let Ok(chs) = crud::list_text_chapters(&handler.conn, &p.id) {
            written_chapters += chs.len();
            for ch in &chs { total_written += ch.word_count; }
        }
    }
    let ops = crud::list_outline_phases(&handler.conn, &novel_id).unwrap_or_default();
    let mut planned = 0usize;
    for p in &ops {
        if let Ok(chs) = crud::list_outline_chapters(&handler.conn, &p.id) {
            planned += chs.len();
        }
    }
    let pct = if novel.total_char > 0 { (total_written as f64 / novel.total_char as f64 * 1000.0).round() / 10.0 } else { 0.0 };
    api_ok(serde_json::json!({
        "novel_name": novel.name, "total_char_target": novel.total_char,
        "chapter_char_target": novel.chapter_char, "total_written": total_written,
        "written_chapters": written_chapters, "planned_chapters": planned,
        "phases": phases.len(), "outline_phases": ops.len(), "completion_pct": pct,
    }))
}

// ─── Agent handlers ───

#[derive(Deserialize)]
struct AgentWriteBody { chapter_id: String, brief: String }

async fn api_agent_write(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AgentWriteBody>,
) -> Json<serde_json::Value> {
    let pp = state.project_path.clone();
    let novel_id = {
        let conn = rusqlite::Connection::open(state.project_path.join("project.db")).unwrap();
        active_novel_id(&conn).unwrap_or_default()
    };
    let cmd = AgentCommand::WriteChapter { novel_id, chapter_id: body.chapter_id, brief: body.brief };
    match tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let conn = rusqlite::Connection::open(pp.join("project.db")).unwrap();
        rt.block_on(pnw_kernel::agent::execute_agent_command(&conn, &pp, cmd))
    }).await.unwrap() {
        Ok(s) => Json(serde_json::json!({"status":"ok","summary":s})),
        Err(e) => Json(serde_json::json!({"status":"error","error":e.to_string()})),
    }
}

#[derive(Deserialize)]
struct AgentReviseBody { chapter_id: String, feedback: String }

async fn api_agent_revise(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AgentReviseBody>,
) -> Json<serde_json::Value> {
    let pp = state.project_path.clone();
    let cmd = AgentCommand::ReviseChapter { chapter_id: body.chapter_id, feedback: body.feedback };
    match tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let conn = rusqlite::Connection::open(pp.join("project.db")).unwrap();
        rt.block_on(pnw_kernel::agent::execute_agent_command(&conn, &pp, cmd))
    }).await.unwrap() {
        Ok(s) => Json(serde_json::json!({"status":"ok","summary":s})),
        Err(e) => Json(serde_json::json!({"status":"error","error":e.to_string()})),
    }
}

async fn api_agent_evaluate(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> Json<serde_json::Value> {
    let pp = state.project_path.clone();
    let cmd = AgentCommand::Evaluate { chapter_id: id };
    match tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let conn = rusqlite::Connection::open(pp.join("project.db")).unwrap();
        rt.block_on(pnw_kernel::agent::execute_agent_command(&conn, &pp, cmd))
    }).await.unwrap() {
        Ok(s) => Json(serde_json::json!({"status":"ok","summary":s})),
        Err(e) => Json(serde_json::json!({"status":"error","error":e.to_string()})),
    }
}

// ─── Export ───

#[derive(Deserialize)]
struct ExportQuery {
    limit: Option<usize>,
}

async fn api_export_txt(State(state): State<Arc<AppState>>, Query(q): Query<ExportQuery>) -> Json<serde_json::Value> {
    let handler = match open_fresh_handler(&state) {
        Ok(h) => h,
        Err(e) => return Json(serde_json::json!({"status":"error","error":e})),
    };
    let novel_id = match active_novel_id(&handler.conn) {
        Ok(id) => id,
        Err(e) => return Json(serde_json::json!({"status":"error","error":e})),
    };
    let novel = match crud::get_novel(&handler.conn, &novel_id) {
        Ok(Some(n)) => n,
        _ => return Json(serde_json::json!({"status":"error","error":"Novel not found"})),
    };
    let phases = crud::list_text_phases(&handler.conn, &novel_id).unwrap_or_default();
    let mut all_chapters = Vec::new();
    for phase in &phases {
        if let Ok(chs) = crud::list_text_chapters(&handler.conn, &phase.id) {
            for ch in chs {
                if let Ok(content) = storage::read_text(&state.project_path.join(&ch.file_path)) {
                    all_chapters.push((phase.name.clone(), ch.name.clone(), ch.sort, content));
                }
            }
        }
    }
    all_chapters.sort_by(|a, b| a.2.cmp(&b.2));
    let count = all_chapters.len();
    let limited = q.limit.map(|l| &all_chapters[..l.min(count)]).unwrap_or(&all_chapters);
    let merged: String = limited.iter().fold(String::new(), |mut acc, (pn, cn, _, c)| {
        acc.push_str(&format!("【{}】{}\n\n{}\n\n", pn, cn, c));
        acc
    });
    let wc = merged.chars().filter(|c| !c.is_whitespace()).count() as u64;
    Json(serde_json::json!({
        "status":"ok",
        "novel_name": novel.name,
        "chapter_count": count,
        "returned_chapters": limited.len(),
        "word_count": wc,
        "content": merged,
    }))
}

// ─── Generic command ───

#[derive(Deserialize)]
struct CmdBody { command: String, #[serde(default)] args: serde_json::Value }

async fn api_command(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CmdBody>,
) -> Json<ApiResponse<serde_json::Value>> {
    let handler = match open_fresh_handler(&state) {
        Ok(h) => h,
        Err(e) => return api_err(e),
    };
    let novel_id = match active_novel_id(&handler.conn) {
        Ok(id) => id,
        Err(e) => return api_err(e),
    };
    let cmd = match body.command.as_str() {
        // Read commands
        "get_outline" => DataCommand::GetOutlineTree { novel_id, phase_id: None },
        "get_novel" => DataCommand::GetNovel { id: novel_id },
        "list_characters" => DataCommand::ListCharacters { novel_id },
        "get_setting" => DataCommand::GetSetting { novel_id },
        "list_samples" => DataCommand::ListSamples { novel_id },
        "get_plugin" => DataCommand::GetPlugin { novel_id },
        "list_outline_phases" => DataCommand::ListOutlinePhases { novel_id },
        "list_outline_chapters" => {
            let pid = body.args.get("phase_id").and_then(|v| v.as_str()).unwrap_or("");
            DataCommand::ListOutlineChapters { phase_id: pid.to_string() }
        }
        // Write commands
        "create_outline_phase" => {
            let name = body.args.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let phases = crud::list_outline_phases(&handler.conn, &novel_id).unwrap_or_default();
            let sort = phases.iter().map(|p| p.sort).max().unwrap_or(-1) + 1;
            DataCommand::CreateOutlinePhase {
                id: uuid::Uuid::new_v4().to_string(), novel_id, sort,
                name, description: String::new(),
            }
        }
        "create_outline_chapter" => {
            let name = body.args.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let pid = body.args.get("phase_id").and_then(|v| v.as_str()).unwrap_or("");
            DataCommand::CreateOutlineChapter {
                id: uuid::Uuid::new_v4().to_string(), phase_id: pid.to_string(),
                sort: 0, chapter_name: name, content: String::new(), hook: String::new(),
            }
        }
        "create_character" => {
            DataCommand::CreateCharacter {
                id: uuid::Uuid::new_v4().to_string(), novel_id,
                name: body.args.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                char_type: body.args.get("char_type").and_then(|v| v.as_i64()).unwrap_or(2) as i32,
                age: body.args.get("age").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                relationship: body.args.get("relationship").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            }
        }
        "write_setting" => {
            DataCommand::WriteSetting {
                novel_id,
                title: body.args.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                inspiration: body.args.get("inspiration").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                description: body.args.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                novel_type: body.args.get("novel_type").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                tags: body.args.get("tags").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default(),
            }
        }
        _ => return api_err(format!("Unknown command: {}", body.command)),
    };
    match handler.execute(cmd) {
        Ok(output) => Json(ApiResponse::from_output(output)),
        Err(e) => api_err(e.to_string()),
    }
}

// ─── Gateway UI (embedded) ───

const GATEWAY_HTML: &str = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>PNW Gateway</title>
<link rel="stylesheet" href="/gateway/style.css">
</head>
<body>
<div id="app">
  <aside id="sidebar">
    <header><h1>📖 PNW</h1><div class="version" id="version"></div></header>
    <nav>
      <button class="nav-item active" data-view="dashboard">📊 总览</button>
      <button class="nav-item" data-view="outline">📂 大纲</button>
      <button class="nav-item" data-view="reader">📖 阅读</button>
      <button class="nav-item" data-view="write">✍ 写作</button>
      <button class="nav-item" data-view="settings">⚙ 设定</button>
      <button class="nav-item" data-view="characters">👤 角色</button>
    </nav>
    <div class="stats-mini" id="stats-mini"></div>
  </aside>
  <main id="content">
    <div id="view-dashboard" class="view active">
      <h2>📊 项目总览</h2>
      <div class="cards" id="dashboard-cards"></div>
    </div>
    <div id="view-outline" class="view">
      <h2>📂 大纲结构</h2>
      <div id="outline-tree"></div>
    </div>
    <div id="view-reader" class="view">
      <h2>📖 阅读</h2>
      <div id="chapter-list"></div>
      <div id="reader-nav" style="display:flex;gap:8px;margin:8px 0">
        <button class="btn secondary" id="btn-prev" onclick="navChapter(-1)">← 上一章</button>
        <button class="btn secondary" id="btn-next" onclick="navChapter(1)">下一章 →</button>
      </div>
      <div id="chapter-content" class="reader-content"></div>
    </div>
    <div id="view-write" class="view">
      <h2>✍ 写作</h2>
      <div id="write-toolbar">
        <select id="write-chapter-select" style="width:100%;padding:8px;background:var(--bg);border:1px solid var(--border);border-radius:4px;color:var(--text);font-size:14px;margin-bottom:12px">
          <option value="">— 选择章节 —</option>
        </select>
        <div id="write-chapter-info" style="font-size:12px;color:var(--dim);margin-bottom:12px"></div>
        <label style="font-size:12px;color:var(--dim);display:block;margin-bottom:4px">写作要求 / 反馈</label>
        <textarea id="write-input" rows="4" style="width:100%;padding:8px;background:var(--bg);border:1px solid var(--border);border-radius:4px;color:var(--text);font-size:14px;resize:vertical;margin-bottom:8px;font-family:inherit" placeholder="输入写作要求或修改意见…"></textarea>
        <div id="write-buttons" style="display:flex;gap:6px;flex-wrap:wrap">
          <button class="btn" onclick="doWrite()">✍ 写正文</button>
          <button class="btn secondary" onclick="doEvaluate()">📊 评估</button>
          <button class="btn secondary" onclick="doRevise()">🔄 修改</button>
        </div>
        <div id="write-status" style="margin-top:12px;padding:12px;border-radius:6px;background:var(--bg2);border:1px solid var(--border);white-space:pre-wrap;font-size:13px;min-height:40px;display:none"></div>
      </div>
    </div>
    <div id="view-settings" class="view">
      <h2>⚙ 设定管理</h2>
      <div id="setting-form"></div>
    </div>
    <div id="view-characters" class="view">
      <h2>👤 角色管理</h2>
      <div id="character-list"></div>
    </div>
  </main>
</div>
<script src="/gateway/app.js"></script>
</body>
</html>"#;

const GATEWAY_CSS: &str = r#"*{margin:0;padding:0;box-sizing:border-box}
:root{--bg:#1a1a2e;--bg2:#16213e;--bg3:#0f3460;--text:#e4e4e7;--dim:#a1a1aa;--accent:#e94560;--border:#2a2a4a;--success:#22c55e;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif}
body{background:var(--bg);color:var(--text);height:100vh;overflow:hidden}
#app{display:grid;grid-template-columns:200px 1fr;height:100vh}
#sidebar{background:var(--bg2);border-right:1px solid var(--border);display:flex;flex-direction:column;padding:12px}
#sidebar header{padding-bottom:12px;border-bottom:1px solid var(--border);margin-bottom:12px}
#sidebar h1{font-size:18px;color:var(--accent)}
.version{font-size:11px;color:var(--dim);margin-top:4px}
.nav-item{display:block;width:100%;padding:10px 12px;margin-bottom:4px;background:none;color:var(--dim);border:none;border-radius:6px;text-align:left;font-size:14px;cursor:pointer}
.nav-item:hover{background:var(--bg3);color:var(--text)}
.nav-item.active{background:var(--accent);color:#fff}
.stats-mini{padding:12px 0;margin-top:auto;font-size:12px;color:var(--dim);border-top:1px solid var(--border)}
#content{padding:24px;overflow-y:auto}
.view{display:none}.view.active{display:block}
h2{font-size:20px;margin-bottom:16px}
.cards{display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:12px}
.card{padding:16px;background:var(--bg2);border:1px solid var(--border);border-radius:8px}
.card h3{font-size:12px;color:var(--dim);margin-bottom:8px;text-transform:uppercase}
.card .value{font-size:24px;font-weight:700}
.phase{padding:8px 0}
.phase-header{padding:8px 12px;font-weight:600;cursor:pointer;color:var(--accent)}
.chapter-item{padding:6px 12px 6px 24px;cursor:pointer;font-size:14px;display:flex;align-items:center;gap:8px}
.chapter-item .dot{width:8px;height:8px;border-radius:50%;display:inline-block}
.dot.done{background:var(--success)}.dot.pending{background:var(--dim);opacity:.4}
.reader-content{padding:20px;background:var(--bg2);border:1px solid var(--border);border-radius:8px;margin-top:12px;line-height:1.8;white-space:pre-wrap;font-size:15px;min-height:200px}
#chapter-list{display:flex;flex-wrap:wrap;gap:6px;margin-bottom:16px}
#chapter-list button{padding:6px 12px;background:var(--bg2);border:1px solid var(--border);border-radius:4px;color:var(--text);font-size:13px;cursor:pointer}
#chapter-list button:hover{border-color:var(--accent)}
.char-item{padding:8px 12px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;margin-bottom:6px;display:flex;gap:12px;align-items:center;font-size:14px}
.char-item .tag{font-size:11px;padding:2px 6px;background:var(--bg3);border-radius:3px;color:var(--dim)}
.setting-field{margin-bottom:12px}
.setting-field label{display:block;font-size:12px;color:var(--dim);margin-bottom:4px}
.setting-field input,.setting-field textarea{width:100%;padding:8px 12px;background:var(--bg);border:1px solid var(--border);border-radius:4px;color:var(--text);font-size:14px}
.setting-field textarea{min-height:80px;resize:vertical}
.btn{padding:8px 16px;background:var(--accent);color:#fff;border:none;border-radius:6px;font-size:14px;cursor:pointer;transition:opacity .15s}
.btn:hover{opacity:.85}.btn.secondary{background:var(--bg3);color:var(--text);border:1px solid var(--border)}.btn.secondary:hover{background:var(--border)}
.progress-bar{height:6px;background:var(--border);border-radius:3px;overflow:hidden;margin-top:8px}
.progress-bar .fill{height:100%;background:var(--accent);border-radius:3px;transition:width .5s}"#;

const GATEWAY_JS: &str = r#"
const BASE = '';
let state = {};

function showError(msg) {
  const el = document.getElementById('error-toast') || (() => {
    const d = document.createElement('div');
    d.id = 'error-toast';
    d.style.cssText = 'position:fixed;top:12px;right:12px;padding:12px 20px;background:#ef4444;color:#fff;border-radius:8px;font-size:13px;z-index:999;max-width:400px;display:none';
    document.body.appendChild(d);
    return d;
  })();
  el.textContent = msg;
  el.style.display = 'block';
  setTimeout(() => { el.style.display = 'none'; }, 5000);
}

async function api(path, opts = {}) {
  try {
    const res = await fetch(BASE + '/api' + path, { headers: { 'Content-Type': 'application/json' }, ...opts });
    const json = await res.json();
    if (json.status === 'error' && json.error) showError(json.error);
    return json;
  } catch (e) {
    showError('请求失败: ' + e.message);
    return { status: 'error', error: e.message };
  }
}

async function loadStats() {
  const res = await api('/stats');
  if (res.status !== 'ok') return;
  document.getElementById('stats-mini').innerHTML =
    `<div>✍ ${res.data.total_written} 字</div><div>📄 ${res.data.written_chapters}/${res.data.planned_chapters} 章</div>`;
  const vs = await api('/status');
  document.getElementById('version').textContent = 'v' + (vs.version || '');
}

async function loadDashboard() {
  const res = await api('/stats');
  if (res.status !== 'ok') return;
  const s = res.data;
  document.getElementById('dashboard-cards').innerHTML = `
    <div class="card"><h3>📖 项目</h3><div class="value">${s.novel_name}</div></div>
    <div class="card"><h3>✍ 已写</h3><div class="value">${s.total_written}</div><div class="sub">目标 ${s.total_char_target}</div>
      <div class="progress-bar"><div class="fill" style="width:${s.completion_pct}%"></div></div></div>
    <div class="card"><h3>📄 章节</h3><div class="value">${s.written_chapters}/${s.planned_chapters}</div><div class="sub">${s.phases} 卷</div></div>`;
}

async function loadOutline() {
  const res = await api('/outline');
  if (res.status !== 'ok' || !res.data?.OutlineTree) return;
  let html = '';
  for (const phase of res.data.OutlineTree) {
    const p = phase.phase || phase;
    const chs = phase.chapters || [];
    html += `<div class="phase"><div class="phase-header">📂 ${p.name}</div>`;
    for (const ch of chs) {
      html += `<div class="chapter-item" onclick="openChapter('${ch.text_chapter_id || ch.id}')">
        <span class="dot ${ch.text_chapter_id ? 'done' : 'pending'}"></span>${ch.chapter_name}</div>`;
    }
    html += `</div>`;
  }
  document.getElementById('outline-tree').innerHTML = html;
}

async function loadReader() {
  const res = await api('/chapters');
  if (res.status !== 'ok') return;
  state.chapters = res.data || [];
  document.getElementById('chapter-list').innerHTML = state.chapters.map(ch =>
    `<button onclick="showChapter('${ch.id}')">${ch.name}</button>`
  ).join('');
  // Also populate writing chapter selector
  const sel = document.getElementById('write-chapter-select');
  if (sel) {
    sel.innerHTML = '<option value="">— 选择章节 —</option>' +
      state.chapters.map(ch => `<option value="${ch.id}">${ch.name} (${ch.word_count}字)</option>`).join('');
  }
}

async function showChapter(id) {
  const res = await api('/chapter/' + id);
  if (res.status !== 'ok') return;
  const ch = res.data;
  state.currentChapterIdx = state.chapters?.findIndex(c => c.id === id) ?? -1;
  document.getElementById('chapter-content').innerHTML =
    `<h3>${ch.name}</h3><p style="color:var(--dim);font-size:12px;margin:8px 0">${ch.word_count} 字 · ${ch.phase_name||''}</p>
     <div class="reader-content">${escHtml(ch.content || '(空)')}</div>`;
  document.querySelectorAll('#chapter-list button').forEach(b => b.style.borderColor = '');
  const btn = document.querySelectorAll('#chapter-list button');
  for (let i = 0; i < btn.length; i++) {
    if (btn[i].textContent.includes(ch.name)) { btn[i].style.borderColor = 'var(--accent)'; break; }
  }
  document.getElementById('btn-prev').style.display = state.currentChapterIdx > 0 ? '' : 'none';
  document.getElementById('btn-next').style.display = state.currentChapterIdx < (state.chapters?.length||0) - 1 ? '' : 'none';
}

function navChapter(dir) {
  const idx = (state.currentChapterIdx ?? -1) + dir;
  if (idx >= 0 && state.chapters && idx < state.chapters.length) {
    showChapter(state.chapters[idx].id);
  }
}

function openChapter(id) { showChapter(id); switchView('reader'); }

// ── Writing functions ──

function getWriteChapterId() {
  const sel = document.getElementById('write-chapter-select');
  return sel?.value || '';
}

function showWriteStatus(msg, isError) {
  const el = document.getElementById('write-status');
  el.style.display = 'block';
  el.style.color = isError ? '#ef4444' : 'var(--text)';
  el.textContent = msg;
}

async function doWrite() {
  const cid = getWriteChapterId();
  if (!cid) { showWriteStatus('请先选择章节', true); return; }
  const brief = document.getElementById('write-input').value;
  if (!brief) { showWriteStatus('请输入写作要求', true); return; }
  showWriteStatus('⏳ Agent B 正在写作中…');
  const res = await api('/agent/write', { method: 'POST', body: JSON.stringify({ chapter_id: cid, brief }) });
  if (res.status === 'ok') {
    showWriteStatus('✅ ' + (res.summary || '写作完成'));
    await loadStats(); await loadReader();
  } else {
    showWriteStatus('❌ ' + (res.error || '写作失败'), true);
  }
}

async function doEvaluate() {
  const cid = getWriteChapterId();
  if (!cid) { showWriteStatus('请先选择章节', true); return; }
  showWriteStatus('⏳ 评估中…');
  const res = await api('/agent/evaluate/' + cid, { method: 'POST' });
  if (res.status === 'ok') {
    showWriteStatus('📊 评估结果:\n' + (res.summary || '完成'));
  } else {
    showWriteStatus('❌ ' + (res.error || '评估失败'), true);
  }
}

async function doRevise() {
  const cid = getWriteChapterId();
  if (!cid) { showWriteStatus('请先选择章节', true); return; }
  const feedback = document.getElementById('write-input').value;
  if (!feedback) { showWriteStatus('请输入修改意见', true); return; }
  showWriteStatus('⏳ Agent B 正在修改…');
  const res = await api('/agent/revise', { method: 'POST', body: JSON.stringify({ chapter_id: cid, feedback }) });
  if (res.status === 'ok') {
    showWriteStatus('✅ 修改完成: ' + (res.summary || ''));
    await loadStats();
  } else {
    showWriteStatus('❌ ' + (res.error || '修改失败'), true);
  }
}

function switchView(name) {
  document.querySelectorAll('.nav-item').forEach(b => b.classList.remove('active'));
  document.querySelectorAll('.view').forEach(v => v.classList.remove('active'));
  document.querySelector(`[data-view="${name}"]`).classList.add('active');
  document.getElementById('view-' + name).classList.add('active');
}

async function loadCharacters() {
  const res = await api('/characters');
  if (res.status !== 'ok') return;
  const chars = res.data?.CharacterList || [];
  const tn = ['男主','女主','其他'];
  document.getElementById('character-list').innerHTML = chars.length
    ? chars.map(c => `<div class="char-item"><strong>${escHtml(c.name)}</strong><span class="tag">${tn[c.char_type]||''}</span><span style="color:var(--dim)">${c.age}岁</span><span style="color:var(--dim)">${escHtml(c.relationship)}</span></div>`).join('')
    : '<p style="color:var(--dim)">暂无角色</p>';
}

async function loadSetting() {
  const res = await api('/setting');
  if (res.status !== 'ok') return;
  const s = res.data?.Setting || {};
  const tn = {0:'都市',1:'玄幻',2:'历史',3:'奇幻',4:'武侠',5:'科幻'};
  document.getElementById('setting-form').innerHTML = `
    <div class="setting-field"><label>书名</label><input id="st-title" value="${esc(s.title)}"/></div>
    <div class="setting-field"><label>灵感来源</label><input id="st-inspiration" value="${esc(s.inspiration)}"/></div>
    <div class="setting-field"><label>作品简介</label><textarea id="st-desc">${esc(s.description)}</textarea></div>
    <div class="setting-field"><label>类型：${tn[s.novel_type] || '未知'}</label></div>
    <div class="setting-field"><label>标签</label><input id="st-tags" value="${(s.tags||[]).join(', ')}"/></div>
    <button class="btn" onclick="saveSetting()">保存</button>`;
}

function esc(s) { return (s || '').replace(/"/g,'&quot;').replace(/</g,'&lt;'); }
function escHtml(s) { return (s || '').replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;'); }

async function saveSetting() {
  await api('/setting', { method: 'POST', body: JSON.stringify({
    title: document.getElementById('st-title').value,
    inspiration: document.getElementById('st-inspiration').value,
    description: document.getElementById('st-desc').value,
    tags: document.getElementById('st-tags').value.split(',').map(s=>s.trim()).filter(Boolean),
  })});
}

document.querySelectorAll('.nav-item').forEach(el => {
  el.addEventListener('click', () => {
    switchView(el.dataset.view);
    if (el.dataset.view === 'reader' || el.dataset.view === 'write') loadReader();
  });
});

(async () => { await loadStats(); await loadDashboard(); await loadOutline(); await loadCharacters(); await loadSetting(); await loadReader(); })();
"#;
