use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct EscalationConfig {
    pub guild_id: i64,
    pub enabled: bool,
    pub warn_threshold: i32,
    pub warn_window_seconds: i64,
    pub timeout_window_seconds: i64,
}
