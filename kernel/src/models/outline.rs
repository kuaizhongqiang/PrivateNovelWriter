use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlinePhase {
    pub id: String,
    pub novel_id: String,
    pub sort: i32,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineChapter {
    pub id: String,
    pub phase_id: String,
    pub sort: i32,
    pub chapter_name: String,
    pub content: String,
    pub hook: String,
    pub text_chapter_id: Option<String>,
}

/// 卷+章树形结构，给 read outline 用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineTree {
    pub phase: OutlinePhase,
    pub chapters: Vec<OutlineChapter>,
}
