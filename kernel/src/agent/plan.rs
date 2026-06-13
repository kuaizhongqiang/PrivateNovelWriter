use std::sync::Arc;

use tokio::sync::Mutex;
use super::llm::{LlmProvider, Message, LlmEvent, LlmError};
use crate::db::crud;
use crate::models::*;
use rusqlite::Connection;

pub async fn execute_plan_outline(
    conn: &Connection,
    llm: &dyn LlmProvider,
    novel_id: &str,
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

    emit("read", "读取小说设定和角色");
    let setting = crud::get_setting(conn, novel_id).map_err(|e| LlmError::Api(format!("DB error: {}", e)))?;
    let characters = crud::list_characters(conn, novel_id).map_err(|e| LlmError::Api(format!("DB error: {}", e)))?;
    let novel = crud::get_novel(conn, novel_id)
        .map_err(|e| LlmError::Api(format!("DB error: {}", e)))?
        .ok_or_else(|| LlmError::Api("Novel not found".into()))?;

    let mut system = String::new();
    system.push_str("你是一个专业的小说大纲规划助手。根据小说设定和角色信息，生成详细的大纲。\n\n");
    system.push_str("输出格式要求:\n- 每卷: [卷] 卷名 | 描述\n- 每章: [章] 章名 | 概要 | 章尾钩子\n- 每行一个，用 | 分隔字段\n\n");

    if let Some(ref s) = setting {
        system.push_str(&format!("标题: {}\n简介: {}\n", s.title, s.description));
        if !s.tags.is_empty() {
            system.push_str(&format!("标签: {}\n", s.tags.join(", ")));
        }
        system.push('\n');
    }
    if !characters.is_empty() {
        system.push_str("角色:\n");
        for c in &characters {
            system.push_str(&format!("- {} ({}): {}\n", c.name,
                match c.char_type.to_i32() { 0 => "男主", 1 => "女主", _ => "配角" },
                c.relationship));
        }
        system.push('\n');
    }

    let user_prompt = format!(
        "规划需求: {}\n\n小说总字数目标: {} 字, 每章约 {} 字\n\n请输出大纲规划结果：",
        brief, novel.total_char, novel.chapter_char
    );

    let messages = vec![
        Message { role: "system".to_string(), content: system },
        Message { role: "user".to_string(), content: user_prompt },
    ];

    emit("llm", "AI 正在规划大纲...");

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
                            let s = llm_sender.clone();
                            tokio::spawn(async move {
                                let g = s.lock().await;
                                if let Some(ref tx) = *g {
                                    tx.send(LlmEvent::Token(t)).ok();
                                }
                            });
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

    emit("write", "保存大纲到数据库");
    let mut phase_count = 0;
    let mut chapter_count = 0;

    for line in content.lines() {
        let line = line.trim();
        if !line.contains('|') { continue; }
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 2 { continue; }
        let raw_name = parts[0].trim();
        let desc = parts[1].trim();
        if raw_name.is_empty() { continue; }

        if raw_name.starts_with("[章]") || raw_name.contains("章") {
            // 章: [章] 章名 | 概要 | 钩子
            let name = raw_name.trim_start_matches("[章]").trim();
            if let Ok(phases) = crud::list_outline_phases(conn, novel_id) {
                if let Some(last) = phases.last() {
                    let oc_id = uuid::Uuid::new_v4().to_string();
                    let chapters = crud::list_outline_chapters(conn, &last.id).ok();
                    let sort = chapters.as_ref().and_then(|c| c.last().map(|ch| ch.sort + 1)).unwrap_or(0);
                    let hook = parts.get(2).map(|s| s.trim()).unwrap_or("");
                    crud::create_outline_chapter(conn, &OutlineChapter {
                        id: oc_id, phase_id: last.id.clone(), sort,
                        chapter_name: name.to_string(), content: desc.to_string(),
                        hook: hook.to_string(), text_chapter_id: None,
                    }).ok();
                    chapter_count += 1;
                }
            }
        } else {
            // 卷: [卷] 卷名 | 描述
            let name = raw_name.trim_start_matches("[卷]").trim();
            let phase_id = uuid::Uuid::new_v4().to_string();
            let phases = crud::list_outline_phases(conn, novel_id).ok();
            let sort = phases.as_ref().and_then(|p| p.last().map(|ph| ph.sort + 1)).unwrap_or(0);
            crud::create_outline_phase(conn, &OutlinePhase {
                id: phase_id, novel_id: novel_id.to_string(), sort,
                name: name.to_string(), description: desc.to_string(),
            }).ok();
            phase_count += 1;
        }
    }

    Ok(format!("已规划大纲: {} 卷, {} 章。", phase_count, chapter_count))
}
