-- Stage channels: channel_type 3 = stage channel
-- Stage sessions represent active speaking events
-- Stage participants track speaker vs audience roles

CREATE TABLE stage_sessions (
    id BIGINT PRIMARY KEY,
    channel_id BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE UNIQUE,
    topic VARCHAR(200),
    started_by BIGINT NOT NULL REFERENCES users(id),
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_stage_sessions_channel ON stage_sessions(channel_id);

-- Stage participants: speakers can talk, audience can raise hand
CREATE TABLE stage_participants (
    id BIGINT PRIMARY KEY,
    channel_id BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(20) NOT NULL DEFAULT 'audience', -- 'speaker' or 'audience'
    hand_raised BOOLEAN NOT NULL DEFAULT FALSE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(channel_id, user_id)
);

CREATE INDEX idx_stage_participants_channel ON stage_participants(channel_id);
