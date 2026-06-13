use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub novel_id: String,
    pub name: String,
    pub plugin_type: PluginType,
    pub description: String,
    pub benefit: String,
    pub cost: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginType {
    System,
    Gift,
    Prop,
    Skill,
}

impl PluginType {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::Gift,
            2 => Self::Prop,
            3 => Self::Skill,
            _ => Self::System,
        }
    }

    pub fn to_i32(&self) -> i32 {
        match self {
            Self::System => 0,
            Self::Gift => 1,
            Self::Prop => 2,
            Self::Skill => 3,
        }
    }
}
