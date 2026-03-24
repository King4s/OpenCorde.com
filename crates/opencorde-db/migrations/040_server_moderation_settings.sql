-- Migration 040: Server moderation settings
-- Adds verification level, content filter, notification defaults, banner, and vanity URL.

ALTER TABLE servers
    ADD COLUMN IF NOT EXISTS verification_level         SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS explicit_content_filter    SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS default_notifications      SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS banner_url                 TEXT,
    ADD COLUMN IF NOT EXISTS vanity_url                 VARCHAR(32) UNIQUE,
    ADD COLUMN IF NOT EXISTS system_channel_id          BIGINT REFERENCES channels(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS rules_channel_id           BIGINT REFERENCES channels(id) ON DELETE SET NULL;

-- verification_level: 0=NONE 1=LOW(email) 2=MEDIUM(5min) 3=HIGH(10min) 4=VERY_HIGH(phone)
-- explicit_content_filter: 0=DISABLED 1=MEMBERS_WITHOUT_ROLES 2=ALL_MEMBERS
-- default_notifications: 0=ALL_MESSAGES 1=ONLY_MENTIONS
