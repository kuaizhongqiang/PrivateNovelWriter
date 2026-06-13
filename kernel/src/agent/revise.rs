use std::path::Path;
use std::sync::Arc;

use crate::db::crud;
use crate::storage;
use tokio::sync::Mutex;

use super::llm::{LlmError, LlmEvent, LlmProvider, Message};
use rusqlite::Connection;

pub async fn execute_revise(
    conn: &Connection,
    project_path: &Path,
    llm: &dyn LlmProvider,
    chapter_id: &str,
    feedback: &str,
    event_sender: Option<&Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<LlmEvent>>>>>,
) -> Result<String, LlmError> {
    let emit = super::make_emit(event_sender);

    emit("read", "读取原文");
    let tc = crud::get_text_chapter(conn, chapter_id)
        .map_err(|e| LlmError::Api(format!("DB error: {}", e)))?
        .ok_or_else(|| LlmError::Api(format!("Text chapter {} not found", chapter_id)))?;

    let full_path = project_path.join(&tc.file_path);
    let original =
        storage::read_text(&full_path).map_err(|e| LlmError::Api(format!("IO error: {}", e)))?;

    let system = "你是一个专业的小说修改助手。根据用户的修改意见，修改给定的正文内容。\n\n要求:\n- 只输出修改后的正文，不要附加说明\n- 保持纯文本格式\n- 保持原文的风格和语气\n- 如果没有指定范围，修改你认为需要改的部分".to_string();
    let user_prompt = format!(
        "原文:\n{}\n\n修改意见:\n{}\n\n请输出修改后的完整正文：",
        original, feedback
    );

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

    emit("llm", "AI 正在修改正文...");

    let content = if let Some(sender) = event_sender {
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
                                    let _ = tx.send(LlmEvent::Token(t));
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

    emit("write", "保存修改结果（先备份原文）");
    let trimmed = content.trim().to_string();
    let wc = storage::count_chars(&trimmed);

    // 备份原文
    if let Ok(orig) = storage::read_text(&full_path) {
        let bak_path = full_path.with_extension("txt.bak");
        storage::write_text(&bak_path, &orig).ok();
    }

    storage::write_text(&full_path, &trimmed)
        .map_err(|e| LlmError::Api(format!("IO error: {}", e)))?;

    let mut updated = tc;
    updated.word_count = wc;
    crud::update_text_chapter(conn, &updated)
        .map_err(|e| LlmError::Api(format!("DB error: {}", e)))?;

    Ok(format!(
        "已修改《{}》，现 {} 字。修改意见: {}",
        updated.name, wc, feedback
    ))
}
