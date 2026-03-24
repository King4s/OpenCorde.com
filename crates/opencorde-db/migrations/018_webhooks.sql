-- Migration: 018_webhooks
-- Adds incoming webhooks for external message posting.

CREATE TABLE webhooks (
    id          BIGINT PRIMARY KEY,
    channel_id  BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    server_id   BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name        VARCHAR(100) NOT NULL DEFAULT 'Webhook',
    token       VARCHAR(64) NOT NULL UNIQUE,
    created_by  BIGINT NOT NULL REFERENCES users(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_webhooks_channel ON webhooks(channel_id);
CREATE INDEX idx_webhooks_server ON webhooks(server_id);
CREATE INDEX idx_webhooks_token ON webhooks(token);
