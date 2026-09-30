-- Migration 046: DM channel unreads
-- Per-user unread message tracking for DM channels.

CREATE TABLE IF NOT EXISTS dm_channel_unreads (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    dm_channel_id   BIGINT NOT NULL REFERENCES dm_channels(id) ON DELETE CASCADE,
    unread_count    INT NOT NULL DEFAULT 1,
    last_message_id BIGINT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, dm_channel_id)
);

CREATE INDEX IF NOT EXISTS idx_dm_unreads_user
    ON dm_channel_unreads (user_id);

CREATE INDEX IF NOT EXISTS idx_dm_unreads_channel
    ON dm_channel_unreads (dm_channel_id);
