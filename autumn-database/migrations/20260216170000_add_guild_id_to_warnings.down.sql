DROP INDEX IF EXISTS warnings_guild_user_warned_at_idx;

ALTER TABLE warnings
    DROP COLUMN IF EXISTS guild_id;
