-- Bridge channel mappings: links Discord channels to OpenCorde channels
-- Supports bidirectional bridging via optional webhook (Discord token for OpenCorde→Discord)

CREATE TABLE IF NOT EXISTS bridge_channel_mappings (
    id                      BIGSERIAL PRIMARY KEY,
    discord_guild_id        BIGINT NOT NULL,
    discord_channel_id      BIGINT NOT NULL UNIQUE,
    -- Webhook for OpenCorde → Discord direction (NULL = one-way Discord→OpenCorde only)
    discord_webhook_id      BIGINT,
    discord_webhook_token   TEXT,
    opencorde_server_id     BIGINT NOT NULL,
    opencorde_channel_id    BIGINT NOT NULL UNIQUE,
    enabled                 BOOLEAN NOT NULL DEFAULT TRUE,
    -- Cursor tracking for incremental processing
    last_discord_msg_id     BIGINT DEFAULT 0,    -- last Discord message processed
    last_opencorde_msg_id   BIGINT DEFAULT 0,    -- last OpenCorde message forwarded to Discord
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_bridge_mapping_discord
    ON bridge_channel_mappings (discord_channel_id)
    WHERE enabled = TRUE;

CREATE INDEX IF NOT EXISTS idx_bridge_mapping_opencorde
    ON bridge_channel_mappings (opencorde_channel_id)
    WHERE enabled = TRUE;
