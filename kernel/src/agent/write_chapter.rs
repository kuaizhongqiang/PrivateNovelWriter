use std::path::Path;
use std::sync::Arc;

use crate::db::crud;
use crate::storage;
use crate::TextChapter;
use tokio::sync::Mutex;

use super::llm::{LlmError, LlmEvent, LlmProvider, Message};
use rusqlite::Connection;

/// 尝试将 ID 解析为正文章节：
/// 1. 先直接查 text_chapter 表
/// 2. 如果找不到，尝试作为 outline_chapter ID 查找
/// 3. 如果找到 outline 且有关联的正文章节，返回之
/// 4. 如果找到 outline 但无关联，自动创建正文章节并建立关联
fn resolve_or_create_text_chapter(
    conn: &Connection,
    project_path: &Path,
    novel_id: &str,
    chapter_id: &str,
) -> Result<TextChapter, LlmError> {
    // 先尝试 resolve
    if let Some(tc) = crud::resolve_text_chapter(conn, chapter_id)
        .map_err(|e| LlmError::Api(format!("DB error: {}", e)))?
    {
        return Ok(tc);
    }

    // 检查是否 outline chapter
    let oc = crud::get_outline_chapter(conn, chapter_id)
        .map_err(|e| LlmError::Api(format!("DB error: {}", e)))?
        .ok_or_else(|| LlmError::Api(format!("Text chapter {} not found", chapter_id)))?;

    // 获取大纲卷（outline_phase）信息
    let outline_phases = crud::list_outline_phases(conn, novel_id)
        .map_err(|e| LlmError::Api(format!("DB error: {}", e)))?;
    let outline_phase = outline_phases
        .iter()
        .find(|p| p.id == oc.phase_id)
        .ok_or_else(|| LlmError::Api("Outline phase not found".into()))?;

    // 查找或创建对应的正文卷（text_phase）
    let text_phases = crud::list_text_phases(conn, novel_id)
        .map_err(|e| LlmError::Api(format!("DB error: {}", e)))?;
    let text_phase_id =
        if let Some(tp) = text_phases.iter().find(|tp| tp.sort == outline_phase.sort) {
            tp.id.clone()
        } else if let Some(tp) = text_phases.first() {
            // 没有 sort 匹配的，用第一个正文卷
            tp.id.clone()
        } else {
            // 没有正文卷，创建一个与大纲卷同名的
            let id = uuid::Uuid::new_v4().to_string();
            let tp = crate::models::TextPhase {
                id: id.clone(),
                novel_id: novel_id.to_string(),
                sort: outline_phase.sort,
                name: outline_phase.name.clone(),
            };
            crud::create_text_phase(conn, &tp)
                .map_err(|e| LlmError::Api(format!("DB error: {}", e)))?;
            id
        };

    // 获取正文卷名用于文件路径
    let phase_name: String = conn
        .query_row(
            "SELECT name FROM text_phase WHERE id = ?1",
            rusqlite::params![text_phase_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "unknown".to_string());

    // 生成新的正文章节
    let tc_id = uuid::Uuid::new_v4().to_string();
    let chapters = crud::list_text_chapters(conn, &text_phase_id)
        .map_err(|e| LlmError::Api(format!("DB error: {}", e)))?;
    let sort = chapters.iter().map(|c| c.sort).max().unwrap_or(-1) + 1;
    let file_path = format!("text/{}/ch-{:03}.txt", phase_name, sort);

    // 确保目录存在
    let full_path = project_path.join(&file_path);
    storage::ensure_dir(&full_path).map_err(|e| LlmError::Api(format!("IO error: {}", e)))?;

    let tc = crate::models::TextChapter {
        id: tc_id.clone(),
        phase_id: text_phase_id,
        sort,
        name: oc.chapter_name.clone(),
        file_path,
        word_count: 0,
    };
    crud::create_text_chapter(conn, &tc).map_err(|e| LlmError::Api(format!("DB error: {}", e)))?;

    // 更新大纲章节的 text_chapter_id
    let mut oc = oc;
    oc.text_chapter_id = Some(tc_id);
    crud::update_outline_chapter(conn, &oc)
        .map_err(|e| LlmError::Api(format!("DB error: {}", e)))?;

    Ok(tc)
}

pub async fn execute_write_chapter(
    conn: &Connection,
    project_path: &Path,
    llm: &dyn LlmProvider,
    novel_id: &str,
    chapter_id: &str,
    brief: &str,
    event_sender: Option<&Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<LlmEvent>>>>>,
) -> Result<String, LlmError> {
    let emit = super::make_emit(event_sender);

    emit("read", "读取正文章节");

    let tc = resolve_or_create_text_chapter(conn, project_path, novel_id, chapter_id)?;

    emit("read", "读取大纲");
    let outline_chapter = crud::find_outline_chapter_by_text_id(conn, novel_id, chapter_id)
        .map_err(|e| LlmError::Api(format!("DB error: {}", e)))?;

    emit("read", "读取角色和设定");
    let characters = crud::list_characters(conn, novel_id)
        .map_err(|e| LlmError::Api(format!("DB error: {}", e)))?;

    let samples = crud::list_samples(conn, novel_id)
        .map_err(|e| LlmError::Api(format!("DB error: {}", e)))?;

    let phase = crud::list_text_phases(conn, novel_id)
        .map_err(|e| LlmError::Api(format!("DB error: {}", e)))?
        .into_iter()
        .find(|p| p.id == tc.phase_id);

    // 拼 prompt
    let mut system = String::new();
    system.push_str(
        "你是一个专业的中长篇小说写作助手。你的职责是根据大纲和设定，生成高质量的正文内容。\n\n",
    );
    system.push_str("写作要求:\n- 正文是纯文本，禁止任何 Markdown 或格式标记\n- 段落间用空行分隔\n- 人物行为符合角色设定\n- 对话体现人物性格\n- 情节推进有因果逻辑\n- 章尾留下悬念或钩子\n\n");

    if !characters.is_empty() {
        system.push_str("角色列表:\n");
        for c in &characters {
            let role = match c.char_type.to_i32() {
                0 => "男主",
                1 => "女主",
                _ => "其他",
            };
            system.push_str(&format!(
                "- {} ({}, {}岁, 关系: {})\n",
                c.name, role, c.age, c.relationship
            ));
        }
        system.push('\n');
    }
    if !samples.is_empty() {
        system.push_str("文风参考样例:\n");
        for s in &samples {
            system.push_str(&format!("[{}]\n{}\n\n", s.title, s.content));
        }
    }

    let mut user_prompt = String::new();
    if let Some((ref p, ref oc)) = outline_chapter {
        user_prompt.push_str(&format!("卷: {}\n卷描述: {}\n\n", p.name, p.description));
        user_prompt.push_str(&format!(
            "章名: {}\n大纲: {}\n",
            oc.chapter_name, oc.content
        ));
        if !oc.hook.is_empty() {
            user_prompt.push_str(&format!("本章钩子: {}\n", oc.hook));
        }
        user_prompt.push('\n');
    }
    if let Some(ref p) = phase {
        user_prompt.push_str(&format!("正文所属卷: {}\n", p.name));
    }
    let target_chars = crud::get_novel(conn, novel_id)
        .ok()
        .flatten()
        .map_or(2000, |n| n.chapter_char as u32);
    user_prompt.push_str(&format!(
        "\n写作要求: {}\n请生成正文，字数在 {} 字左右。",
        brief, target_chars
    ));

    let messages = vec![
        Message {
            role: "system".to_string(),
            content: system,
        },
        Message {
            role: "user".to_string(),
            content: user_prompt,
        },
    ];

    emit("llm", "AI 正在生成正文...");

    let content = if let Some(sender) = event_sender {
        // 流式
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let llm_sender = sender.clone();
        let llm_fut = llm.chat_stream(&messages, &[], tx);

        tokio::pin!(llm_fut);

        let mut full = String::new();
        loop {
            tokio::select! {
                msg = rx.recv() => {
                    match msg {
                        Some(LlmEvent::Token(t)) => {
                            full.push_str(&t);
                            if let Ok(g) = llm_sender.try_lock() {
                                if let Some(ref tx) = *g {
                                    tx.send(LlmEvent::Token(t)).ok();
                                }
                            }
                        }
                        Some(LlmEvent::Thinking(t)) => {
                            if let Ok(g) = llm_sender.try_lock() {
                                if let Some(ref tx) = *g {
                                    tx.send(LlmEvent::Thinking(t)).ok();
                                }
                            }
                        }
                        Some(LlmEvent::Done) | None => break,
                        _ => {}
                    }
                }
                result = &mut llm_fut => {
                    result?;
                }
            }
        }
        full
    } else {
        let resp = llm.chat(&messages, &[]).await?;
        resp.content
    };

    if content.trim().is_empty() {
        return Err(LlmError::Api("LLM returned empty content".into()));
    }

    emit("write", "保存正文到文件");
    let full_path = project_path.join(&tc.file_path);
    storage::ensure_dir(&full_path).map_err(|e| LlmError::Api(format!("IO error: {}", e)))?;
    let trimmed = content.trim().to_string();
    let wc = storage::count_chars(&trimmed);
    storage::write_text(&full_path, &trimmed)
        .map_err(|e| LlmError::Api(format!("IO error: {}", e)))?;

    let mut updated = tc;
    updated.word_count = wc;
    crud::update_text_chapter(conn, &updated)
        .map_err(|e| LlmError::Api(format!("DB error: {}", e)))?;

    Ok(format!("生成了《{}》正文，共 {} 字。", updated.name, wc))
}
