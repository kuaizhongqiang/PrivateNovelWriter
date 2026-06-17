use std::path::Path;
use std::sync::Arc;

use crate::db::crud;
use crate::storage;
use tokio::sync::Mutex;

use super::llm::{LlmError, LlmEvent, LlmProvider, Message};
use rusqlite::Connection;

pub async fn execute_evaluate(
    conn: &Connection,
    project_path: &Path,
    llm: &dyn LlmProvider,
    chapter_id: &str,
    event_sender: Option<&Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<LlmEvent>>>>>,
) -> Result<String, LlmError> {
    let emit = super::make_emit(event_sender);

    emit("read", "读取正文章节");
    let tc = crud::resolve_text_chapter(conn, chapter_id)
        .map_err(|e| LlmError::Api(format!("DB error: {}", e)))?
        .ok_or_else(|| LlmError::Api(format!("Text chapter {} not found", chapter_id)))?;

    let full_path = project_path.join(&tc.file_path);
    let content =
        storage::read_text(&full_path).map_err(|e| LlmError::Api(format!("IO error: {}", e)))?;

    let system = "你是一个专业的小说编辑。请从以下维度评估正文质量:\n\n1. 人物塑造（人物行为是否一致，对话是否符合性格）\n2. 情节推进（节奏是否合理，因果逻辑是否清晰）\n3. 文笔表达（语言流畅度，描写生动性）\n4. 钩子设计（章尾是否有悬念，是否能吸引读者继续看）\n\n每个维度一行: 维度名: 评分/10 | 评语\n最后一行: 总体: 评分/10 | 总结".to_string();
    let user_prompt = format!("请评估以下正文:\n\n{}", content);
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

    emit("llm", "AI 正在评估...");

    let result = if let Some(sender) = event_sender {
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

    if result.trim().is_empty() {
        return Err(LlmError::Api("LLM returned empty evaluation".into()));
    }
    Ok(format!("《{}》评估结果:\n{}", tc.name, result.trim()))
}
