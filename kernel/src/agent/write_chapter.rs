use std::path::Path;
use std::sync::Arc;

use crate::db::crud;
use crate::storage;
use tokio::sync::Mutex;

use super::llm::{LlmProvider, Message, LlmEvent, LlmError};
use rusqlite::Connection;

pub async fn execute_write_chapter(
    conn: &Connection,
    project_path: &Path,
    llm: &dyn LlmProvider,
    novel_id: &str,
    chapter_id: &str,
    brief: &str,
    event_sender: Option<&Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<LlmEvent>>>>>,
) -> Result<String, LlmError> {
    let emit = |name: &str, status: &str| {
        if let Some(sender) = event_sender {
            let n = name.to_string();
            let s = status.to_string();
            let sender = sender.clone();
            tokio::spawn(async move {
                let guard = sender.lock().await;
                if let Some(ref tx) = *guard {
                    tx.send(LlmEvent::Step { name: n, status: s }).ok();
                }
            });
        }
    };

    emit("read", "读取正文章节");

    let tc = crud::get_text_chapter(conn, chapter_id)
        .map_err(|e| LlmError::Api(format!("DB error: {}", e)))?
        .ok_or_else(|| LlmError::Api(format!("Text chapter {} not found", chapter_id)))?;

    emit("read", "读取大纲");
    let outline_chapter = {
        let phases = crud::list_outline_phases(conn, novel_id)
            .map_err(|e| LlmError::Api(format!("DB error: {}", e)))?;
        let mut found = None;
        for phase in &phases {
            let chapters = crud::list_outline_chapters(conn, &phase.id)
                .map_err(|e| LlmError::Api(format!("DB error: {}", e)))?;
            for oc in &chapters {
                if oc.text_chapter_id.as_deref() == Some(chapter_id) {
                    found = Some((phase.clone(), oc.clone()));
                    break;
                }
            }
        }
        found
    };

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
    system.push_str("你是一个专业的中长篇小说写作助手。你的职责是根据大纲和设定，生成高质量的正文内容。\n\n");
    system.push_str("写作要求:\n- 正文是纯文本，禁止任何 Markdown 或格式标记\n- 段落间用空行分隔\n- 人物行为符合角色设定\n- 对话体现人物性格\n- 情节推进有因果逻辑\n- 章尾留下悬念或钩子\n\n");

    if !characters.is_empty() {
        system.push_str("角色列表:\n");
        for c in &characters {
            let role = match c.char_type.to_i32() {
                0 => "男主", 1 => "女主", _ => "其他",
            };
            system.push_str(&format!("- {} ({}, {}岁, 关系: {})\n", c.name, role, c.age, c.relationship));
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
        user_prompt.push_str(&format!("章名: {}\n大纲: {}\n", oc.chapter_name, oc.content));
        if !oc.hook.is_empty() {
            user_prompt.push_str(&format!("本章钩子: {}\n", oc.hook));
        }
        user_prompt.push('\n');
    }
    if let Some(ref p) = phase {
        user_prompt.push_str(&format!("正文所属卷: {}\n", p.name));
    }
    let target_chars = crud::get_novel(conn, novel_id).ok().flatten().map_or(2000, |n| n.chapter_char as u32);
    user_prompt.push_str(&format!("\n写作要求: {}\n请生成正文，字数在 {} 字左右。", brief, target_chars));

    let messages = vec![
        Message { role: "system".to_string(), content: system },
        Message { role: "user".to_string(), content: user_prompt },
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
                            if let Ok(mut g) = llm_sender.try_lock() {
                                if let Some(ref tx) = *g {
                                    tx.send(LlmEvent::Token(t)).ok();
                                }
                            }
                        }
                        Some(LlmEvent::Thinking(t)) => {
                            if let Ok(mut g) = llm_sender.try_lock() {
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
    storage::write_text(&full_path, &trimmed).map_err(|e| LlmError::Api(format!("IO error: {}", e)))?;

    let mut updated = tc;
    updated.word_count = wc;
    crud::update_text_chapter(conn, &updated).map_err(|e| LlmError::Api(format!("DB error: {}", e)))?;

    Ok(format!("生成了《{}》正文，共 {} 字。", updated.name, wc))
}
