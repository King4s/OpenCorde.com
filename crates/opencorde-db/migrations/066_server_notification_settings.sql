-- Migration 066: Server-level notification settings
-- Allows users to set per-server notification defaults, mute servers,
-- and suppress specific mention types (everyone, here, role mentions).
--
-- level: 0 = ALL_MESSAGES (default), 1 = ONLY_MENTIONS, 2 = NOTHING (fully muted)
-- mute_until: if set and in the future, all notifications are suppressed
-- suppress_everyone: suppress @everyone mentions
-- suppress_here: suppress @here mentions
-- suppress_role_mentions: suppress @role mentions

CREATE TABLE IF NOT EXISTS server_notification_settings (
    user_id               BIGINT       NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    server_id             BIGINT       NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    level                 SMALLINT     NOT NULL DEFAULT 0,
    mute_until            TIMESTAMPTZ,
    suppress_everyone     BOOLEAN      NOT NULL DEFAULT FALSE,
    suppress_here         BOOLEAN      NOT NULL DEFAULT FALSE,
    suppress_role_mentions BOOLEAN     NOT NULL DEFAULT FALSE,
    updated_at            TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, server_id)
);

CREATE INDEX IF NOT EXISTS idx_server_notif_user ON server_notification_settings (user_id);
CREATE INDEX IF NOT EXISTS idx_server_notif_server ON server_notification_settings (server_id);
