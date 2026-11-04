-- Add forwarded_from_id to messages for message forwarding/sharing
ALTER TABLE messages ADD COLUMN forwarded_from_id BIGINT REFERENCES messages(id) ON DELETE SET NULL;
CREATE INDEX idx_messages_forwarded_from ON messages (forwarded_from_id) WHERE forwarded_from_id IS NOT NULL;
