/// 创作命令：需要 Agent B 编排执行
#[derive(Debug, Clone)]
pub enum AgentCommand {
    /// 写正文：Agent 读大纲+设定 → LLM 生成 → 写入
    WriteChapter {
        novel_id: String,
        chapter_id: String,
        brief: String,
    },
    /// 修改正文：Agent 读原文+反馈 → LLM 修改 → 写入
    ReviseChapter {
        chapter_id: String,
        feedback: String,
    },
    /// 规划大纲：Agent 根据设定+请求规划一卷的章大纲
    PlanOutline { novel_id: String, brief: String },
    /// 评估写作品质
    Evaluate { chapter_id: String },
}
