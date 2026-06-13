pub mod llm;
pub mod write_chapter;
pub mod revise;
pub mod plan;
pub mod evaluate;

use std::path::Path;
use std::sync::Arc;

use rusqlite::Connection;
use tokio::sync::Mutex;

use llm::{create_provider_from_env, LlmError, LlmEvent};
use crate::command::agent::AgentCommand;

/// 执行创作命令 (Agent B 编排), 非流式
pub async fn execute_agent_command(
    conn: &Connection,
    project_path: &Path,
    cmd: AgentCommand,
) -> Result<String, LlmError> {
    let llm = create_provider_from_env()?;
    let novel_id = get_active_novel_id(conn)?;

    match cmd {
        AgentCommand::WriteChapter { chapter_id, brief, .. } => {
            write_chapter::execute_write_chapter(
                conn, project_path, llm.as_ref(), &novel_id, &chapter_id, &brief, None,
            ).await
        }
        AgentCommand::ReviseChapter { chapter_id, feedback } => {
            revise::execute_revise(
                conn, project_path, llm.as_ref(), &chapter_id, &feedback, None,
            ).await
        }
        AgentCommand::PlanOutline { brief, .. } => {
            plan::execute_plan_outline(
                conn, llm.as_ref(), &novel_id, &brief, None,
            ).await
        }
        AgentCommand::Evaluate { chapter_id } => {
            evaluate::execute_evaluate(
                conn, project_path, llm.as_ref(), &chapter_id, None,
            ).await
        }
    }
}

/// 执行创作命令 (流式, 通过 sender 发送事件)
pub async fn execute_agent_command_stream(
    conn: &Connection,
    project_path: &Path,
    cmd: AgentCommand,
    sender: tokio::sync::mpsc::UnboundedSender<LlmEvent>,
) -> Result<String, LlmError> {
    let llm = create_provider_from_env()?;
    let novel_id = get_active_novel_id(conn)?;
    let sender = Arc::new(Mutex::new(Some(sender)));

    let emit = |name: &str, status: &str| {
        let sender = sender.clone();
        let n = name.to_string();
        let s = status.to_string();
        async move {
            let guard = sender.lock().await;
            if let Some(ref tx) = *guard {
                tx.send(LlmEvent::Step { name: n, status: s }).ok();
            }
        }
    };

    // 每个 agent 函数接收 Option<Sender>，有则发流式事件
    match cmd {
        AgentCommand::WriteChapter { chapter_id, brief, .. } => {
            emit("write_chapter", "开始写作").await;
            let result = write_chapter::execute_write_chapter(
                conn, project_path, llm.as_ref(), &novel_id, &chapter_id, &brief, Some(&sender),
            ).await;
            let mut guard = sender.lock().await;
            guard.take(); // 关闭 sender
            result
        }
        AgentCommand::ReviseChapter { chapter_id, feedback } => {
            emit("revise", "开始修改").await;
            let result = revise::execute_revise(
                conn, project_path, llm.as_ref(), &chapter_id, &feedback, Some(&sender),
            ).await;
            let mut guard = sender.lock().await;
            guard.take();
            result
        }
        AgentCommand::PlanOutline { brief, .. } => {
            emit("plan", "开始规划大纲").await;
            let result = plan::execute_plan_outline(
                conn, llm.as_ref(), &novel_id, &brief, Some(&sender),
            ).await;
            let mut guard = sender.lock().await;
            guard.take();
            result
        }
        AgentCommand::Evaluate { chapter_id } => {
            emit("evaluate", "开始评估").await;
            let result = evaluate::execute_evaluate(
                conn, project_path, llm.as_ref(), &chapter_id, Some(&sender),
            ).await;
            let mut guard = sender.lock().await;
            guard.take();
            result
        }
    }
}

fn get_active_novel_id(conn: &Connection) -> Result<String, LlmError> {
    use crate::db::crud;
    let list = crud::list_novels(conn).map_err(|e| LlmError::Api(format!("DB error: {}", e)))?;
    for novel in &list {
        if novel.active {
            return Ok(novel.id.clone());
        }
    }
    list.first().map(|n| n.id.clone()).ok_or_else(|| LlmError::Api("No novels found".into()))
}
