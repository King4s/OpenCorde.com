-- Add reply threading to messages
ALTER TABLE messages ADD COLUMN reply_to_id BIGINT REFERENCES messages(id) ON DELETE SET NULL;
CREATE INDEX idx_messages_reply_to ON messages (reply_to_id) WHERE reply_to_id IS NOT NULL;
