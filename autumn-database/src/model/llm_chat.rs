#[derive(Clone, Debug)]
pub struct LlmChatEntry {
    pub user_id: u64,
    pub display_name: Option<String>,
    pub role: String,
    pub content: String,
    pub created_at: u64,
}
