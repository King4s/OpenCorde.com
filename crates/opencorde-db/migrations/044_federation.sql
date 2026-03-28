-- Migration 044: Federation channel mapping
-- Marks channels as federated and stores the origin server + channel reference.
-- When a message is posted to a federated channel, it is forwarded to all active peers.
-- Receiving servers look up the local channel by matching the topic field:
--   channel.topic LIKE 'federated:{origin_hostname}:{origin_channel_id}%'

ALTER TABLE channels
    ADD COLUMN IF NOT EXISTS federated BOOLEAN NOT NULL DEFAULT false;

COMMENT ON COLUMN channels.federated IS
    'True if messages in this channel are forwarded to all active mesh peers';

-- Log of outbound federation events (for debugging + deduplication)
CREATE TABLE IF NOT EXISTS federation_event_log (
    id            BIGINT PRIMARY KEY,
    event_type    TEXT        NOT NULL,
    origin        TEXT        NOT NULL,
    destination   TEXT        NOT NULL,
    payload       JSONB       NOT NULL,
    sent_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    success       BOOLEAN     NOT NULL DEFAULT true
);

CREATE INDEX IF NOT EXISTS idx_federation_event_log_origin
    ON federation_event_log (origin, sent_at DESC);
