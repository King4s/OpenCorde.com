-- Create channels table
-- channel_type: 0=Text, 1=Voice, 2=Category

CREATE TABLE channels (
    id           BIGINT PRIMARY KEY,
    server_id    BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name         VARCHAR(100) NOT NULL,
    channel_type SMALLINT NOT NULL DEFAULT 0,
    topic        TEXT,
    position     INT NOT NULL DEFAULT 0,
    parent_id    BIGINT REFERENCES channels(id) ON DELETE SET NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_channels_server ON channels (server_id);
