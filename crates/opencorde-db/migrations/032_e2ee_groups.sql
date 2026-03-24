-- Migration 032: E2EE group state per channel per user
-- Each row stores one member's MLS group state for one channel.
-- The welcome_message column is populated when the initiator adds this user;
-- it is read once on join and cleared afterwards.

CREATE TABLE IF NOT EXISTS e2ee_groups (
    id              BIGSERIAL    PRIMARY KEY,
    channel_id      BIGINT       NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    user_id         BIGINT       NOT NULL REFERENCES users(id)    ON DELETE CASCADE,
    group_state     BYTEA        NOT NULL,   -- TLS-serialized MLS group state (updated on every commit)
    welcome_message BYTEA,                  -- Welcome bytes for this user (NULL after consumed)
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    UNIQUE(channel_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_e2ee_groups_channel ON e2ee_groups(channel_id);
CREATE INDEX IF NOT EXISTS idx_e2ee_groups_user    ON e2ee_groups(user_id);
