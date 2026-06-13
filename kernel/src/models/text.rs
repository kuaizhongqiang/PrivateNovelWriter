use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPhase {
    pub id: String,
    pub novel_id: String,
    pub sort: i32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChapter {
    pub id: String,
    pub phase_id: String,
    pub sort: i32,
    pub name: String,
    pub file_path: String,
    pub word_count: i32,
}
