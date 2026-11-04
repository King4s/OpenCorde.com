-- Migration 062: Storage quota controls
-- Adds per-user and instance-wide storage quota columns,
-- plus an instance_settings singleton table for defaults.

-- 1. Add quota columns to users
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS storage_quota_bytes     BIGINT,   -- NULL = use instance default
    ADD COLUMN IF NOT EXISTS storage_used_bytes_cache BIGINT NOT NULL DEFAULT 0;

-- 2. Create instance_settings singleton table
CREATE TABLE IF NOT EXISTS instance_settings (
    id                             BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id = TRUE),
    default_user_quota_bytes       BIGINT NOT NULL DEFAULT 104857600,   -- 100 MB
    instance_total_storage_cap_bytes BIGINT,                             -- NULL = no cap
    created_at                     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Singleton constraint
CREATE UNIQUE INDEX IF NOT EXISTS idx_instance_settings_singleton ON instance_settings ((TRUE));

-- Auto-update trigger
CREATE OR REPLACE FUNCTION update_instance_settings_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_instance_settings_updated_at ON instance_settings;
CREATE TRIGGER trg_instance_settings_updated_at
    BEFORE UPDATE ON instance_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_instance_settings_updated_at();

-- Insert default row if the table was empty
INSERT INTO instance_settings (id, default_user_quota_bytes)
VALUES (TRUE, 104857600)
ON CONFLICT (id) DO NOTHING;
