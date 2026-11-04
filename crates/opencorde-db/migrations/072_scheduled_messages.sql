-- Migration: 072_scheduled_messages
-- Scheduled/send-later messages. Users can compose a message, pick a future
-- delivery time, and have it injected into the target channel at that time.

CREATE TABLE scheduled_messages (
    id           BIGINT PRIMARY KEY,
    author_id    BIGINT NOT NULL REFERENCES users(id),
    channel_id   BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    content      TEXT NOT NULL DEFAULT '',
    attachments  JSONB NOT NULL DEFAULT '[]',
    reply_to_id  BIGINT REFERENCES messages(id) ON DELETE SET NULL,
    scheduled_at TIMESTAMPTZ NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delivered    BOOLEAN NOT NULL DEFAULT FALSE,
    delivered_at TIMESTAMPTZ,
    cancelled    BOOLEAN NOT NULL DEFAULT FALSE,
    cancelled_at TIMESTAMPTZ
);

CREATE INDEX idx_scheduled_messages_author ON scheduled_messages (author_id, scheduled_at ASC) WHERE NOT delivered AND NOT cancelled;
CREATE INDEX idx_scheduled_messages_due ON scheduled_messages (scheduled_at ASC) WHERE NOT delivered AND NOT cancelled;
