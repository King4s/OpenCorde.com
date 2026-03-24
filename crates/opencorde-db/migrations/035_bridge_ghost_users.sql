-- Bridge ghost users: OpenCorde user accounts that mirror Discord users
-- Ghost users are created automatically when Discord messages arrive.
-- They have no email/password — they exist only as display identities.

CREATE TABLE IF NOT EXISTS bridge_ghost_users (
    id                  BIGSERIAL PRIMARY KEY,
    discord_user_id     BIGINT NOT NULL UNIQUE,
    opencorde_user_id   BIGINT NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    discord_username    VARCHAR(64) NOT NULL,
    discord_avatar_url  TEXT,
    last_seen           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_bridge_ghost_discord  ON bridge_ghost_users (discord_user_id);
CREATE INDEX IF NOT EXISTS idx_bridge_ghost_opencorde ON bridge_ghost_users (opencorde_user_id);
