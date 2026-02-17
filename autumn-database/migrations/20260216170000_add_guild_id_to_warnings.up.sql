ALTER TABLE warnings
    ADD COLUMN IF NOT EXISTS guild_id BIGINT NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS warnings_guild_user_warned_at_idx
    ON warnings (guild_id, user_id, warned_at DESC);
