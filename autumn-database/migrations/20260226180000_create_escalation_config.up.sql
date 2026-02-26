CREATE TABLE IF NOT EXISTS escalation_config (
    guild_id           BIGINT PRIMARY KEY,
    enabled            BOOLEAN NOT NULL DEFAULT FALSE,
    warn_threshold     INT NOT NULL DEFAULT 3,
    warn_window_seconds BIGINT NOT NULL DEFAULT 86400,
    timeout_window_seconds BIGINT NOT NULL DEFAULT 604800
);
