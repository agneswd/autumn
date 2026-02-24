CREATE TABLE IF NOT EXISTS guild_ai_config (
    guild_id BIGINT PRIMARY KEY,
    llm_enabled BOOLEAN NOT NULL DEFAULT TRUE
);
