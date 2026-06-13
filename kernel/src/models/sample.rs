use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailSample {
    pub id: String,
    pub novel_id: String,
    pub title: String,
    pub content: String,
}
