-- Create voice_states table
-- One row per active voice connection

CREATE TABLE voice_states (
    user_id     BIGINT NOT NULL REFERENCES users(id),
    channel_id  BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    session_id  VARCHAR(64) NOT NULL,
    self_mute   BOOLEAN NOT NULL DEFAULT false,
    self_deaf   BOOLEAN NOT NULL DEFAULT false,
    joined_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id)
);

CREATE INDEX idx_voice_channel ON voice_states (channel_id);
