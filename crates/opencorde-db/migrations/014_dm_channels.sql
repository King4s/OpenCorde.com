-- Direct message channels between two users
CREATE TABLE dm_channels (
    id          BIGINT PRIMARY KEY,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Members of DM channels (always exactly 2 for 1-on-1, up to 10 for group DMs)
CREATE TABLE dm_channel_members (
    dm_channel_id   BIGINT NOT NULL REFERENCES dm_channels(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    last_read_id    BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (dm_channel_id, user_id)
);

-- Messages in DM channels (separate from server channel messages)
CREATE TABLE dm_messages (
    id          BIGINT PRIMARY KEY,
    dm_id       BIGINT NOT NULL REFERENCES dm_channels(id) ON DELETE CASCADE,
    author_id   BIGINT NOT NULL REFERENCES users(id),
    content     TEXT NOT NULL DEFAULT '',
    attachments JSONB NOT NULL DEFAULT '[]',
    edited_at   TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_dm_messages_channel ON dm_messages (dm_id, id DESC);
CREATE INDEX idx_dm_members_user ON dm_channel_members (user_id);
