use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WordFilterConfig {
    pub guild_id: u64,
    pub enabled: bool,
    pub action: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WordFilterWord {
    pub id: u64,
    pub guild_id: u64,
    pub word: String,
    pub is_preset: bool,
    pub created_at: u64,
}
