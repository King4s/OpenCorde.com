-- Add poll support to messages
-- A poll is a JSONB object stored on the message row.
-- JSON structure:
-- {
--   "question": "string",
--   "answers": [{"id": 0, "text": "string", "votes": [user_id_int64, ...]}, ...],
--   "allow_multiselect": false,
--   "expires_at": "ISO8601 timestamptz string | null",
--   "closed": false
-- }

ALTER TABLE messages ADD COLUMN IF NOT EXISTS poll JSONB;

CREATE INDEX IF NOT EXISTS idx_messages_poll ON messages USING gin (poll) WHERE poll IS NOT NULL;
