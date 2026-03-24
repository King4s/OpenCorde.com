-- Migration: 017_threads
-- Adds thread sub-conversations attached to channel messages.

CREATE TABLE threads (
    id            BIGINT PRIMARY KEY,
    channel_id    BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    parent_msg_id BIGINT REFERENCES messages(id) ON DELETE SET NULL,
    name          VARCHAR(100) NOT NULL DEFAULT 'Thread',
    created_by    BIGINT NOT NULL REFERENCES users(id),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_msg_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    msg_count     INT NOT NULL DEFAULT 0
);

CREATE INDEX idx_threads_channel ON threads(channel_id, last_msg_at DESC);
CREATE INDEX idx_threads_parent_msg ON threads(parent_msg_id);

ALTER TABLE messages ADD COLUMN thread_id BIGINT REFERENCES threads(id) ON DELETE CASCADE;
CREATE INDEX idx_messages_thread ON messages(thread_id, id DESC);
