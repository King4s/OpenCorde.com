-- Migration 046: Per-channel notification preferences
-- Allows users to override the server-level notification setting per channel.
-- level: 0=ALL_MESSAGES (default), 1=ONLY_MENTIONS, 2=MUTED (no notifications)

CREATE TABLE IF NOT EXISTS channel_notification_settings (
    user_id    BIGINT   NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id BIGINT   NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    level      SMALLINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, channel_id)
);

CREATE INDEX IF NOT EXISTS idx_channel_notif_user ON channel_notification_settings (user_id);
