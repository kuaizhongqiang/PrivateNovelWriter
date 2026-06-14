use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use pnw_kernel::command::data::DataCommand;
use pnw_kernel::db::{crud, schema};
use pnw_kernel::handler::{Handler, Output};
use pnw_kernel::storage;

mod server;

fn get_project_path() -> PathBuf {
    if let Ok(p) = std::env::var("PNW_PROJECT") {
        return PathBuf::from(p);
    }
    // 默认在当前目录找 .pnw 文件或 project.db
    let cwd = std::env::current_dir().expect("Cannot get current dir");
    if cwd.join("project.db").exists() {
        return cwd;
    }
    cwd
}

fn open_db(project_path: &Path) -> rusqlite::Result<rusqlite::Connection> {
    let db_path = project_path.join("project.db");
    let conn = rusqlite::Connection::open(&db_path)?;
    schema::init_schema(&conn)?;
    Ok(conn)
}

fn print_json<T: serde::Serialize>(val: &T) {
    println!("{}", serde_json::to_string_pretty(val).unwrap());
}

/// 检查 npm 更新（1 天缓存）
async fn check_update() {
    const CURRENT: &str = env!("CARGO_PKG_VERSION");
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    let cache_file = std::path::PathBuf::from(home).join(".pnw-update");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // 检查缓存（24小时内不重复查）
    if let Ok(content) = std::fs::read_to_string(&cache_file) {
        if let Ok(last) = content.trim().parse::<u64>() {
            if now - last < 86400 {
                return;
            }
        }
    }

    // 异步查 npm registry
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok();
    if let Some(client) = client {
        if let Ok(resp) = client
            .get("https://registry.npmjs.org/private-novel-writer/latest")
            .send()
            .await
        {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(latest) = json["version"].as_str() {
                    if latest != CURRENT {
                        eprintln!(
                            "\x1b[33m⚠ 新版本可用: v{} (当前 v{})\x1b[0m\n  \x1b[2m更新: npm install -g private-novel-writer@alpha\x1b[0m\n",
                            latest, CURRENT
                        );
                    }
                }
            }
        }
    }
    // 写缓存
    if let Some(parent) = cache_file.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let _ = std::fs::write(&cache_file, now.to_string());
}

// ─── CLI args ───

#[derive(Parser)]
#[command(name = "pnw", about = "PrivateNovelWriter - 私人小说写作助手")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 小说管理
    Novel {
        #[command(subcommand)]
        action: NovelCommands,
    },
    /// 世界观设定
    Setting {
        #[command(subcommand)]
        action: SettingCommands,
    },
    /// 角色管理
    Character {
        #[command(subcommand)]
        action: CharacterCommands,
    },
    /// 系统/金手指设定
    Plugin {
        #[command(subcommand)]
        action: PluginCommands,
    },
    /// 大纲管理
    Outline {
        #[command(subcommand)]
        action: OutlineCommands,
    },
    /// 正文管理
    Text {
        #[command(subcommand)]
        action: TextCommands,
    },
    /// 文风样例管理
    Sample {
        #[command(subcommand)]
        action: SampleCommands,
    },
    /// 全局进度统计
    Status,
    /// 创作命令：写正文 (Agent B)
    Chapter {
        #[command(subcommand)]
        action: ChapterAgentCommands,
    },
    /// 创作命令：评估正文 (Agent B)
    Evaluate {
        /// 正文章节 ID
        id: String,
    },
    /// HTTP 服务模式
    Server {
        /// 监听地址
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// 监听端口
        #[arg(long, default_value_t = 3000)]
        port: u16,
        /// 项目路径
        #[arg(long)]
        project: Option<String>,
        /// CORS 允许的来源（默认 *，生产环境应指定具体域名）
        #[arg(long, default_value = "*")]
        cors_origin: String,
    },
    /// 导出
    Export {
        #[command(subcommand)]
        action: ExportCommands,
    },
}

// ─── Agent-level Chapter Commands ───

#[derive(Subcommand)]
enum ChapterAgentCommands {
    /// 写正文 (Agent B 读大纲+角色 → LLM 生成 → 写入)
    Write {
        id: String,
        #[arg(short, long)]
        btw: String,
    },
    /// 修改正文 (Agent B 读原文+反馈 → LLM 修改 → 写入)
    Revise {
        id: String,
        #[arg(short, long)]
        feedback: String,
    },
}

// ─── Novel ───

#[derive(Subcommand)]
enum NovelCommands {
    /// 新建小说项目
    New { name: String },
    /// 打开已有项目
    Open { path: String },
    /// 列出所有小说
    List,
    /// 查看小说的详细信息
    Show,
    /// 更新小说配置
    Config {
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        total_char: Option<i32>,
        #[arg(long)]
        chapter_char: Option<i32>,
        #[arg(long)]
        sensitivity: Option<i32>,
    },
}

// ─── Setting ───

#[derive(Subcommand)]
enum SettingCommands {
    /// 查看世界观设定
    Show,
    /// 更新世界观设定
    Update {
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        inspiration: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        novel_type: Option<i32>,
        #[arg(long)]
        tags: Option<String>, // JSON array
    },
}

// ─── Character ───

#[derive(Subcommand)]
enum CharacterCommands {
    /// 添加角色
    Add {
        name: String,
        #[arg(long, default_value_t = 2)]
        char_type: i32,
        #[arg(long, default_value_t = 0)]
        age: i32,
        #[arg(long, default_value = "")]
        relationship: String,
    },
    /// 列出所有角色
    List,
    /// 查看角色详情
    Get { id: String },
    /// 更新角色
    Update {
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        char_type: Option<i32>,
        #[arg(long)]
        age: Option<i32>,
        #[arg(long)]
        relationship: Option<String>,
    },
    /// 删除角色
    Delete { id: String },
}

// ─── Plugin ───

#[derive(Subcommand)]
enum PluginCommands {
    /// 查看系统设定
    Show,
    /// 设置系统设定
    Set {
        name: String,
        #[arg(long, default_value_t = 0)]
        plugin_type: i32,
        #[arg(long, default_value = "")]
        description: String,
        #[arg(long, default_value = "")]
        benefit: String,
        #[arg(long, default_value = "")]
        cost: String,
    },
    /// 删除系统设定
    Delete,
}

// ─── Outline ───

#[derive(Subcommand)]
enum OutlineCommands {
    /// 展示完整大纲树
    Show,
    /// 卷管理
    Phase {
        #[command(subcommand)]
        action: OutlinePhaseCommands,
    },
    /// 章管理
    Chapter {
        #[command(subcommand)]
        action: OutlineChapterCommands,
    },
}

#[derive(Subcommand)]
enum OutlinePhaseCommands {
    /// 添加卷
    Add {
        name: String,
        #[arg(long)]
        description: Option<String>,
    },
    /// 列出卷
    List,
    /// 删除卷
    Delete { id: String },
}

#[derive(Subcommand)]
enum OutlineChapterCommands {
    /// 添加章大纲
    Add {
        phase_id: String,
        name: String,
        #[arg(long)]
        content: Option<String>,
        #[arg(long)]
        hook: Option<String>,
    },
    /// 列出章大纲
    List { phase_id: String },
    /// 查看章大纲
    Get { id: String },
    /// 删除章大纲
    Delete { id: String },
    /// 局部修改章大纲
    Patch {
        id: String,
        field: String,
        old_text: String,
        new_text: String,
    },
}

// ─── Text ───

#[derive(Subcommand)]
enum TextCommands {
    /// 卷管理
    Phase {
        #[command(subcommand)]
        action: TextPhaseCommands,
    },
    /// 章管理
    Chapter {
        #[command(subcommand)]
        action: TextChapterCommands,
    },
}

#[derive(Subcommand)]
enum TextPhaseCommands {
    /// 创建正文卷
    Create {
        novel_id: String,
        name: String,
        #[arg(long, default_value_t = 0)]
        sort: i32,
    },
    /// 列出正文卷
    List { novel_id: String },
    /// 删除正文卷
    Delete { phase_id: String },
}

#[derive(Subcommand)]
enum TextChapterCommands {
    /// 创建正文章节
    Create {
        phase_id: String,
        #[arg(long)]
        from_outline: String,
        #[arg(long)]
        name: String,
    },
    /// 写入正文内容
    Write {
        id: String,
        #[arg(long)]
        file: Option<String>,
        #[arg(long)]
        text: Option<String>,
    },
    /// 读取正文内容
    Read { id: String },
    /// 列出正文章节
    List { phase_id: String },
    /// 局部修改正文
    Patch {
        id: String,
        old_text: String,
        new_text: String,
    },
    /// 删除正文章节
    Delete { id: String },
}

// ─── Sample ───

#[derive(Subcommand)]
enum SampleCommands {
    /// 添加文风样例
    Add { title: String, content: String },
    /// 列出文风样例
    List,
    /// 删除文风样例
    Delete { id: String },
}

// ─── Export ───

#[derive(Subcommand)]
enum ExportCommands {
    /// 合并正文章节为一个 .txt 文件
    Txt {
        /// 输出文件路径（默认 {小说名}.txt）
        #[arg(long)]
        output: Option<String>,
    },
}

// ─── Main ───

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    check_update().await;
    let cli = Cli::parse();

    match &cli.command {
        Commands::Novel { action } => handle_novel(action),
        // Agent 命令需要异步执行
        Commands::Chapter { action } => handle_chapter_agent(action).await,
        Commands::Evaluate { id } => handle_evaluate(id).await,
        Commands::Server {
            host,
            port,
            project,
            cors_origin,
        } => {
            server::run_server(host, *port, project.as_deref(), cors_origin).await;
        }
        other => {
            let project_path = get_project_path();
            let conn = open_db(&project_path).expect("Cannot open project database");
            let mut handler = Handler::new(conn, project_path);

            let result = match other {
                Commands::Novel { .. } | Commands::Server { .. } => unreachable!(),
                Commands::Chapter { .. } | Commands::Evaluate { .. } => unreachable!(),
                Commands::Setting { action } => handle_setting(&mut handler, action),
                Commands::Character { action } => handle_character(&mut handler, action),
                Commands::Plugin { action } => handle_plugin(&mut handler, action),
                Commands::Outline { action } => handle_outline(&mut handler, action),
                Commands::Text { action } => handle_text(&mut handler, action),
                Commands::Sample { action } => handle_sample(&mut handler, action),
                Commands::Export { action } => handle_export(&mut handler, action),
                Commands::Status => handle_status(&mut handler),
            };

            match result {
                Ok(output) => {
                    print_json(&output);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

// ─── Novel handlers ───

fn handle_novel(cmd: &NovelCommands) {
    match cmd {
        NovelCommands::New { name } => {
            let dir = std::env::current_dir().unwrap().join(name);
            std::fs::create_dir_all(&dir).expect("Cannot create project directory");
            std::fs::create_dir_all(dir.join("text")).ok();

            let conn = open_db(&dir).expect("Cannot create database");
            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();

            use pnw_kernel::models::{Novel, Sensitivity};
            let novel = Novel {
                id: id.clone(),
                name: name.clone(),
                created: now.clone(),
                modified: now,
                active: true,
                total_char: 0,
                chapter_char: 2000,
                sensitivity: Sensitivity::Normal,
            };
            crud::create_novel(&conn, &novel).expect("Cannot create novel");

            println!("Created novel '{}' at {}", name, dir.display());
            println!("Set PNW_PROJECT={}", dir.display());
            println!("Or run: cd {} && pnw ...", dir.display());
        }
        NovelCommands::Open { path } => {
            let dir = PathBuf::from(path);
            if !dir.join("project.db").exists() {
                eprintln!("Error: no project.db found in {}", dir.display());
                std::process::exit(1);
            }
            println!("{}", dir.canonicalize().unwrap_or(dir).display());
        }
        NovelCommands::List => {
            // 尝试从当前目录找 project.db
            let project_path = get_project_path();
            if let Ok(conn) = open_db(&project_path) {
                if let Ok(list) = crud::list_novels(&conn) {
                    print_json(&list);
                    return;
                }
            }
            println!("[]");
        }
        NovelCommands::Show => {
            let project_path = get_project_path();
            let conn = open_db(&project_path).expect("Cannot open database");
            let handler = Handler::new(conn, project_path);
            let novel_id = get_active_novel_id(&handler.conn);
            if let Ok(output) = handler.execute(DataCommand::GetNovel { id: novel_id }) {
                print_json(&output);
            } else {
                eprintln!("Error: could not load novel");
                std::process::exit(1);
            }
        }
        NovelCommands::Config {
            name,
            total_char,
            chapter_char,
            sensitivity,
        } => {
            let project_path = get_project_path();
            let conn = open_db(&project_path).expect("Cannot open database");
            let novel_id = get_active_novel_id(&conn);
            let handler = Handler::new(conn, project_path);
            let cmd = DataCommand::UpdateNovel {
                id: novel_id,
                name: name.clone(),
                total_char: *total_char,
                chapter_char: *chapter_char,
                sensitivity: *sensitivity,
            };
            handler.execute(cmd).expect("Update failed");
            println!("Config updated");
        }
    }
}

// ─── Setting handlers ───

fn handle_setting(
    handler: &mut Handler,
    cmd: &SettingCommands,
) -> Result<Output, pnw_kernel::handler::HandlerError> {
    let novel_id = get_active_novel_id(&handler.conn);
    match cmd {
        SettingCommands::Show => handler.execute(DataCommand::GetSetting { novel_id }),
        SettingCommands::Update {
            title,
            inspiration,
            description,
            novel_type,
            tags,
        } => {
            // 先读现有 setting
            let existing = crud::get_setting(&handler.conn, &novel_id).ok().flatten();
            let t = title
                .clone()
                .unwrap_or(existing.as_ref().map_or(String::new(), |s| s.title.clone()));
            let i = inspiration.clone().unwrap_or(
                existing
                    .as_ref()
                    .map_or(String::new(), |s| s.inspiration.clone()),
            );
            let d = description.clone().unwrap_or(
                existing
                    .as_ref()
                    .map_or(String::new(), |s| s.description.clone()),
            );
            let nt = novel_type.unwrap_or(existing.as_ref().map_or(0, |s| s.novel_type.to_i32()));
            let tg: Vec<String> = tags
                .as_ref()
                .map(|s| serde_json::from_str(s).unwrap_or_default())
                .unwrap_or_else(|| existing.as_ref().map_or(vec![], |s| s.tags.clone()));

            handler.execute(DataCommand::WriteSetting {
                novel_id,
                title: t,
                inspiration: i,
                description: d,
                novel_type: nt,
                tags: tg,
            })
        }
    }
}

// ─── Character handlers ───

fn handle_character(
    handler: &mut Handler,
    cmd: &CharacterCommands,
) -> Result<Output, pnw_kernel::handler::HandlerError> {
    let novel_id = get_active_novel_id(&handler.conn);
    match cmd {
        CharacterCommands::Add {
            name,
            char_type,
            age,
            relationship,
        } => {
            let id = uuid::Uuid::new_v4().to_string();
            handler.execute(DataCommand::CreateCharacter {
                id,
                novel_id,
                name: name.clone(),
                char_type: *char_type,
                age: *age,
                relationship: relationship.clone(),
            })
        }
        CharacterCommands::List => handler.execute(DataCommand::ListCharacters { novel_id }),
        CharacterCommands::Get { id } => {
            handler.execute(DataCommand::GetCharacter { id: id.clone() })
        }
        CharacterCommands::Update {
            id,
            name,
            char_type,
            age,
            relationship,
        } => {
            // 先读现有
            let existing = crud::get_character(&handler.conn, id)
                .ok()
                .flatten()
                .ok_or_else(|| {
                    pnw_kernel::handler::HandlerError::NotFound(format!("Character {}", id))
                })?;
            handler.execute(DataCommand::UpdateCharacter {
                id: id.clone(),
                novel_id: existing.novel_id,
                name: name.clone().unwrap_or(existing.name),
                char_type: char_type.unwrap_or(existing.char_type.to_i32()),
                age: age.unwrap_or(existing.age),
                relationship: relationship.clone().unwrap_or(existing.relationship),
            })
        }
        CharacterCommands::Delete { id } => {
            handler.execute(DataCommand::DeleteCharacter { id: id.clone() })
        }
    }
}

// ─── Plugin handlers ───

fn handle_plugin(
    handler: &mut Handler,
    cmd: &PluginCommands,
) -> Result<Output, pnw_kernel::handler::HandlerError> {
    let novel_id = get_active_novel_id(&handler.conn);
    match cmd {
        PluginCommands::Show => handler.execute(DataCommand::GetPlugin { novel_id }),
        PluginCommands::Set {
            name,
            plugin_type,
            description,
            benefit,
            cost,
        } => handler.execute(DataCommand::WritePlugin {
            novel_id,
            name: name.clone(),
            plugin_type: *plugin_type,
            description: description.clone(),
            benefit: benefit.clone(),
            cost: cost.clone(),
        }),
        PluginCommands::Delete => handler.execute(DataCommand::DeletePlugin { novel_id }),
    }
}

// ─── Outline handlers ───

fn handle_outline(
    handler: &mut Handler,
    cmd: &OutlineCommands,
) -> Result<Output, pnw_kernel::handler::HandlerError> {
    let novel_id = get_active_novel_id(&handler.conn);
    match cmd {
        OutlineCommands::Show => handler.execute(DataCommand::GetOutlineTree {
            novel_id,
            phase_id: None,
        }),
        OutlineCommands::Phase { action } => match action {
            OutlinePhaseCommands::Add { name, description } => {
                let id = uuid::Uuid::new_v4().to_string();
                // 获取当前最大 sort
                let phases = crud::list_outline_phases(&handler.conn, &novel_id)?;
                let sort = phases.iter().map(|p| p.sort).max().unwrap_or(-1) + 1;
                handler.execute(DataCommand::CreateOutlinePhase {
                    id,
                    novel_id,
                    sort,
                    name: name.clone(),
                    description: description.clone().unwrap_or_default(),
                })
            }
            OutlinePhaseCommands::List => {
                handler.execute(DataCommand::ListOutlinePhases { novel_id })
            }
            OutlinePhaseCommands::Delete { id } => {
                handler.execute(DataCommand::DeleteOutlinePhase {
                    phase_id: id.clone(),
                })
            }
        },
        OutlineCommands::Chapter { action } => match action {
            OutlineChapterCommands::Add {
                phase_id,
                name,
                content,
                hook,
            } => {
                let id = uuid::Uuid::new_v4().to_string();
                // 获取当前最大 sort
                let chapters = crud::list_outline_chapters(&handler.conn, phase_id)?;
                let sort = chapters.iter().map(|c| c.sort).max().unwrap_or(-1) + 1;
                handler.execute(DataCommand::CreateOutlineChapter {
                    id,
                    phase_id: phase_id.clone(),
                    sort,
                    chapter_name: name.clone(),
                    content: content.clone().unwrap_or_default(),
                    hook: hook.clone().unwrap_or_default(),
                })
            }
            OutlineChapterCommands::List { phase_id } => {
                handler.execute(DataCommand::ListOutlineChapters {
                    phase_id: phase_id.clone(),
                })
            }
            OutlineChapterCommands::Get { id } => {
                handler.execute(DataCommand::GetOutlineChapter { id: id.clone() })
            }
            OutlineChapterCommands::Delete { id } => {
                handler.execute(DataCommand::DeleteOutlineChapter { id: id.clone() })
            }
            OutlineChapterCommands::Patch {
                id,
                field,
                old_text,
                new_text,
            } => handler.execute(DataCommand::PatchOutlineChapter {
                chapter_id: id.clone(),
                field: field.clone(),
                old_text: old_text.clone(),
                new_text: new_text.clone(),
            }),
        },
    }
}

// ─── Text handlers ───

fn handle_text(
    handler: &mut Handler,
    cmd: &TextCommands,
) -> Result<Output, pnw_kernel::handler::HandlerError> {
    match cmd {
        TextCommands::Phase { action } => match action {
            TextPhaseCommands::Create {
                novel_id,
                name,
                sort,
            } => {
                let id = uuid::Uuid::new_v4().to_string();
                handler.execute(DataCommand::CreateTextPhase {
                    id,
                    novel_id: novel_id.clone(),
                    sort: *sort,
                    name: name.clone(),
                })
            }
            TextPhaseCommands::List { novel_id } => handler.execute(DataCommand::ListTextPhases {
                novel_id: novel_id.clone(),
            }),
            TextPhaseCommands::Delete { phase_id } => {
                handler.execute(DataCommand::DeleteTextPhase {
                    phase_id: phase_id.clone(),
                })
            }
        },
        TextCommands::Chapter { action } => match action {
            TextChapterCommands::Create {
                phase_id,
                from_outline,
                name,
            } => {
                let id = uuid::Uuid::new_v4().to_string();
                let chapters = crud::list_text_chapters(&handler.conn, phase_id)?;
                let sort = chapters.iter().map(|c| c.sort).max().unwrap_or(-1) + 1;

                // 从 text_phase 表取卷名用于构建文件路径
                let phase_name: String = handler
                    .conn
                    .query_row(
                        "SELECT name FROM text_phase WHERE id = ?1",
                        rusqlite::params![phase_id],
                        |row| row.get(0),
                    )
                    .unwrap_or_else(|_| "unknown".to_string());
                let file_path = format!("text/{}/ch-{:03}.txt", phase_name, sort);
                let full_path = handler.project_path.join(&file_path);

                // 确保目录存在
                if let Some(parent) = full_path.parent() {
                    std::fs::create_dir_all(parent)
                        .unwrap_or_else(|e| panic!("无法创建目录 {}: {}", parent.display(), e));
                }

                let cmd = DataCommand::CreateTextChapter {
                    id: id.clone(),
                    phase_id: phase_id.clone(),
                    sort,
                    name: name.clone(),
                    file_path,
                };
                handler.execute(cmd)?;

                // 更新大纲章节的 text_chapter_id
                let outline_chapter_id = from_outline.clone();
                if let Ok(Some(mut oc)) =
                    crud::get_outline_chapter(&handler.conn, &outline_chapter_id)
                {
                    oc.text_chapter_id = Some(id.clone());
                    let _ = crud::update_outline_chapter(&handler.conn, &oc);
                }

                Ok(Output::Status(format!("Created text chapter: {}", name)))
            }
            TextChapterCommands::Write { id, file, text } => {
                let content = if let Some(f) = file {
                    std::fs::read_to_string(f).map_err(|e| {
                        pnw_kernel::handler::HandlerError::Storage(
                            pnw_kernel::storage::StorageError::Io(e),
                        )
                    })?
                } else if let Some(t) = text {
                    t.clone()
                } else {
                    // 从 stdin 读取
                    let mut buf = String::new();
                    std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf).ok();
                    buf
                };

                let tc = crud::get_text_chapter(&handler.conn, id)?.ok_or_else(|| {
                    pnw_kernel::handler::HandlerError::NotFound(format!("Text chapter {}", id))
                })?;

                let full_path = handler.project_path.join(&tc.file_path);
                storage::write_text(&full_path, &content)?;
                let wc = storage::count_chars(&content);

                let mut updated = tc;
                updated.word_count = wc;
                crud::update_text_chapter(&handler.conn, &updated)?;

                Ok(Output::Status(format!(
                    "Written {} chars to chapter {}",
                    wc, id
                )))
            }
            TextChapterCommands::Read { id } => {
                let tc = crud::get_text_chapter(&handler.conn, id)?.ok_or_else(|| {
                    pnw_kernel::handler::HandlerError::NotFound(format!("Text chapter {}", id))
                })?;
                let full_path = handler.project_path.join(&tc.file_path);
                let content = storage::read_text(&full_path)?;
                Ok(Output::TextChapterWithContent {
                    chapter: tc,
                    content,
                })
            }
            TextChapterCommands::List { phase_id } => {
                handler.execute(DataCommand::ListTextChapters {
                    phase_id: phase_id.clone(),
                })
            }
            TextChapterCommands::Patch {
                id,
                old_text,
                new_text,
            } => handler.execute(DataCommand::PatchTextChapter {
                chapter_id: id.clone(),
                old_text: old_text.clone(),
                new_text: new_text.clone(),
            }),
            TextChapterCommands::Delete { id } => {
                let tc = crud::get_text_chapter(&handler.conn, id)?.ok_or_else(|| {
                    pnw_kernel::handler::HandlerError::NotFound(format!("Text chapter {}", id))
                })?;
                handler.execute(DataCommand::DeleteTextChapter {
                    id: id.clone(),
                    file_path: tc.file_path,
                })
            }
        },
    }
}

// ─── Sample handlers ───

fn handle_sample(
    handler: &mut Handler,
    cmd: &SampleCommands,
) -> Result<Output, pnw_kernel::handler::HandlerError> {
    let novel_id = get_active_novel_id(&handler.conn);
    match cmd {
        SampleCommands::Add { title, content } => {
            let id = uuid::Uuid::new_v4().to_string();
            handler.execute(DataCommand::CreateSample {
                id,
                novel_id,
                title: title.clone(),
                content: content.clone(),
            })
        }
        SampleCommands::List => handler.execute(DataCommand::ListSamples { novel_id }),
        SampleCommands::Delete { id } => {
            handler.execute(DataCommand::DeleteSample { id: id.clone() })
        }
    }
}

// ─── Export handlers ───

fn handle_export(
    handler: &mut Handler,
    cmd: &ExportCommands,
) -> Result<Output, pnw_kernel::handler::HandlerError> {
    match cmd {
        ExportCommands::Txt { output } => {
            let novel_id = get_active_novel_id(&handler.conn);
            let novel = crud::get_novel(&handler.conn, &novel_id)?
                .ok_or_else(|| pnw_kernel::handler::HandlerError::NotFound("Novel".into()))?;

            let phases = crud::list_text_phases(&handler.conn, &novel_id)?;
            let mut all_chapters = Vec::new();
            for phase in &phases {
                let chapters = crud::list_text_chapters(&handler.conn, &phase.id)?;
                for ch in chapters {
                    let full_path = handler.project_path.join(&ch.file_path);
                    if let Ok(content) = storage::read_text(&full_path) {
                        all_chapters.push((phase.name.clone(), ch.name.clone(), ch.sort, content));
                    }
                }
            }

            all_chapters.sort_by(|a, b| a.2.cmp(&b.2));

            let mut merged = String::new();
            for (phase_name, ch_name, _sort, content) in &all_chapters {
                merged.push_str(&format!("【{}】{}\n\n", phase_name, ch_name));
                merged.push_str(content);
                merged.push_str("\n\n");
            }

            let first_sort = all_chapters.first().map(|c| c.2).unwrap_or(0);
            let last_sort = all_chapters.last().map(|c| c.2).unwrap_or(0);
            let default_name = format!("{}_{}——{}", novel.name, first_sort, last_sort);
            let out_path = output
                .as_ref()
                .map(|p| std::path::PathBuf::from(p))
                .unwrap_or_else(|| {
                    let mut p = std::env::current_dir().unwrap_or_default();
                    p.push(format!("{}.txt", default_name));
                    p
                });

            // Use merge_write for atomic export
            let tmp_path = out_path.with_extension("tmp");
            std::fs::write(&tmp_path, &merged).map_err(|e| {
                pnw_kernel::handler::HandlerError::Storage(pnw_kernel::storage::StorageError::Io(e))
            })?;
            std::fs::rename(&tmp_path, &out_path).map_err(|e| {
                pnw_kernel::handler::HandlerError::Storage(pnw_kernel::storage::StorageError::Io(e))
            })?;

            Ok(Output::Status(format!(
                "导出完成: {} ({} 章, {} 字)",
                out_path.display(),
                all_chapters.len(),
                merged.chars().filter(|c| !c.is_whitespace()).count(),
            )))
        }
    }
}

// ─── Status ───

fn handle_status(handler: &mut Handler) -> Result<Output, pnw_kernel::handler::HandlerError> {
    let novel_id = get_active_novel_id(&handler.conn);
    let novel = crud::get_novel(&handler.conn, &novel_id)?.ok_or_else(|| {
        pnw_kernel::handler::HandlerError::NotFound(format!("Novel {}", novel_id))
    })?;

    // 统计所有正文章节字数
    let phases = crud::list_text_phases(&handler.conn, &novel_id)?;
    let mut total_written = 0;
    let mut total_chapters = 0;
    for phase in &phases {
        let chapters = crud::list_text_chapters(&handler.conn, &phase.id)?;
        total_chapters += chapters.len() as i32;
        for ch in &chapters {
            total_written += ch.word_count;
        }
    }

    // 大纲章节数
    let outline_phases = crud::list_outline_phases(&handler.conn, &novel_id)?;
    let mut total_planned = 0;
    for op in &outline_phases {
        let chapters = crud::list_outline_chapters(&handler.conn, &op.id)?;
        total_planned += chapters.len() as i32;
    }

    let status = serde_json::json!({
        "novel": {
            "id": novel.id,
            "name": novel.name,
            "total_char_target": novel.total_char,
            "chapter_char_target": novel.chapter_char,
        },
        "progress": {
            "planned_chapters": total_planned,
            "written_chapters": total_chapters,
            "total_written_chars": total_written,
            "remaining_chars": (novel.total_char - total_written).max(0),
            "completion_pct": if novel.total_char > 0 {
                format!("{:.1}%", (total_written as f64 / novel.total_char as f64) * 100.0)
            } else {
                "N/A".to_string()
            },
        },
        "phases": phases.len(),
        "outline_phases": outline_phases.len(),
    });

    Ok(Output::StatusJson(status))
}

// ─── Helpers ───

fn get_active_novel_id(conn: &rusqlite::Connection) -> String {
    if let Ok(list) = crud::list_novels(conn) {
        if let Some(novel) = list.into_iter().find(|n| n.active) {
            return novel.id;
        }
        // 没 active 则取第一个
        if let Ok(list) = crud::list_novels(conn) {
            if let Some(novel) = list.into_iter().next() {
                return novel.id;
            }
        }
    }
    eprintln!("Error: no novels found. Create one with `pnw novel new <name>`");
    std::process::exit(1);
}

// ─── Agent Command Handlers (async) ───

async fn handle_chapter_agent(cmd: &ChapterAgentCommands) {
    let project_path = get_project_path();
    let conn = open_db(&project_path).expect("Cannot open project database");
    let novel_id = get_active_novel_id(&conn);

    let agent_cmd = match cmd {
        ChapterAgentCommands::Write { id, btw } => {
            pnw_kernel::command::agent::AgentCommand::WriteChapter {
                novel_id,
                chapter_id: id.clone(),
                brief: btw.clone(),
            }
        }
        ChapterAgentCommands::Revise { id, feedback } => {
            pnw_kernel::command::agent::AgentCommand::ReviseChapter {
                chapter_id: id.clone(),
                feedback: feedback.clone(),
            }
        }
    };

    match pnw_kernel::agent::execute_agent_command(&conn, &project_path, agent_cmd).await {
        Ok(summary) => {
            let result = serde_json::json!({ "status": "ok", "summary": summary });
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
        }
        Err(e) => {
            let result = serde_json::json!({ "status": "error", "error": e.to_string() });
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
            std::process::exit(1);
        }
    }
}

async fn handle_evaluate(id: &str) {
    let project_path = get_project_path();
    let conn = open_db(&project_path).expect("Cannot open project database");

    let cmd = pnw_kernel::command::agent::AgentCommand::Evaluate {
        chapter_id: id.to_string(),
    };

    match pnw_kernel::agent::execute_agent_command(&conn, &project_path, cmd).await {
        Ok(summary) => {
            let result = serde_json::json!({ "status": "ok", "summary": summary });
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
        }
        Err(e) => {
            let result = serde_json::json!({ "status": "error", "error": e.to_string() });
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
            std::process::exit(1);
        }
    }
}
