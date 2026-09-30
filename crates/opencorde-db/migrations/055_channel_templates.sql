-- Create channel_templates table
-- Stores reusable channel configurations per server.
--
-- Depends On: servers, channels (for schema context)

CREATE TABLE channel_templates (
    id                BIGINT PRIMARY KEY,
    server_id         BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name              VARCHAR(100) NOT NULL,
    channel_type      SMALLINT NOT NULL DEFAULT 0,
    topic             TEXT,
    nsfw              BOOLEAN NOT NULL DEFAULT false,
    slowmode_delay    INT NOT NULL DEFAULT 0,
    e2ee_enabled      BOOLEAN NOT NULL DEFAULT false,
    synced_with_category BOOLEAN NOT NULL DEFAULT false,
    -- JSON array of {target_type, target_id, allow_bits, deny_bits}
    permission_overrides JSONB NOT NULL DEFAULT '[]',
    created_by        BIGINT NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_channel_templates_server ON channel_templates (server_id);
