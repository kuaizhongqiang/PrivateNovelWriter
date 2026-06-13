use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Novel {
    pub id: String,
    pub name: String,
    pub created: String,
    pub modified: String,
    pub active: bool,
    pub total_char: i32,
    pub chapter_char: i32,
    pub sensitivity: Sensitivity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelSetting {
    pub novel_id: String,
    pub title: String,
    pub inspiration: String,
    pub description: String,
    pub novel_type: NovelType,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelConfig {
    pub total_char: i32,
    pub chapter_char: i32,
    pub sensitivity: Sensitivity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Sensitivity {
    Normal,
    Mixed,
    Porn,
}

impl Sensitivity {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::Mixed,
            2 => Self::Porn,
            _ => Self::Normal,
        }
    }

    pub fn to_i32(&self) -> i32 {
        match self {
            Self::Normal => 0,
            Self::Mixed => 1,
            Self::Porn => 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NovelType {
    Urban,
    Xuanhuan,
    History,
    Fantasy,
    Wuxia,
    SciFi,
}

impl NovelType {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::Xuanhuan,
            2 => Self::History,
            3 => Self::Fantasy,
            4 => Self::Wuxia,
            5 => Self::SciFi,
            _ => Self::Urban,
        }
    }

    pub fn to_i32(&self) -> i32 {
        match self {
            Self::Urban => 0,
            Self::Xuanhuan => 1,
            Self::History => 2,
            Self::Fantasy => 3,
            Self::Wuxia => 4,
            Self::SciFi => 5,
        }
    }
}
