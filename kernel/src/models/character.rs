use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub novel_id: String,
    pub name: String,
    pub char_type: CharacterType,
    pub age: i32,
    pub relationship: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CharacterType {
    Actor,
    Actress,
    Other,
}

impl CharacterType {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::Actress,
            2 => Self::Other,
            _ => Self::Actor,
        }
    }

    pub fn to_i32(&self) -> i32 {
        match self {
            Self::Actor => 0,
            Self::Actress => 1,
            Self::Other => 2,
        }
    }
}
