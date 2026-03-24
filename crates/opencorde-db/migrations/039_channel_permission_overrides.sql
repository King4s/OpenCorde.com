-- Channel permission overrides: per-role and per-user permission grants/denials
CREATE TABLE IF NOT EXISTS channel_permission_overrides (
    id          BIGSERIAL PRIMARY KEY,
    channel_id  BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    target_type TEXT NOT NULL CHECK (target_type IN ('role', 'member')),
    target_id   BIGINT NOT NULL,
    allow_bits  BIGINT NOT NULL DEFAULT 0,
    deny_bits   BIGINT NOT NULL DEFAULT 0,
    UNIQUE(channel_id, target_type, target_id)
);
CREATE INDEX IF NOT EXISTS idx_chan_perm_channel ON channel_permission_overrides(channel_id);
