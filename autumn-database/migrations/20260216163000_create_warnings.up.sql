CREATE TABLE IF NOT EXISTS warnings (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    moderator_id BIGINT NOT NULL,
    reason TEXT NOT NULL,
    warned_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS warnings_user_id_warned_at_idx
    ON warnings (user_id, warned_at DESC);
