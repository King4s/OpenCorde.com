-- Track last-read message per user per channel for unread counting
CREATE TABLE channel_read_state (
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id      BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    last_read_id    BIGINT NOT NULL DEFAULT 0,
    mention_count   INT NOT NULL DEFAULT 0,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, channel_id)
);

CREATE INDEX idx_read_state_user ON channel_read_state (user_id);
