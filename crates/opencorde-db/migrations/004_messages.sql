-- Create messages table
-- Snowflake ID encodes timestamp, so created_at is derived but stored for queries

CREATE TABLE messages (
    id          BIGINT PRIMARY KEY,
    channel_id  BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    author_id   BIGINT NOT NULL REFERENCES users(id),
    content     TEXT NOT NULL DEFAULT '',
    attachments JSONB NOT NULL DEFAULT '[]',
    edited_at   TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_messages_channel ON messages (channel_id, id DESC);
CREATE INDEX idx_messages_author ON messages (author_id);
