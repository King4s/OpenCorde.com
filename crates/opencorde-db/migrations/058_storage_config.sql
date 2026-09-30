-- Migration 058: Storage provider configuration
-- Stores object storage (MinIO/S3) settings for the instance.
-- Only one row should ever exist (singleton pattern).
CREATE TABLE IF NOT EXISTS storage_config (
    id              BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id = TRUE),
    provider        TEXT NOT NULL DEFAULT 'minio',
    endpoint        TEXT NOT NULL DEFAULT 'http://localhost:9000',
    access_key      TEXT,
    secret_key      TEXT,
    bucket          TEXT NOT NULL DEFAULT 'opencorde',
    region          TEXT NOT NULL DEFAULT 'us-east-1',
    force_path_style BOOLEAN NOT NULL DEFAULT TRUE,
    use_ssl         BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Ensure only one row exists
CREATE UNIQUE INDEX IF NOT EXISTS idx_storage_config_singleton ON storage_config ((TRUE));

-- Trigger to auto-update updated_at
CREATE OR REPLACE FUNCTION update_storage_config_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_storage_config_updated_at ON storage_config;
CREATE TRIGGER trg_storage_config_updated_at
    BEFORE UPDATE ON storage_config
    FOR EACH ROW
    EXECUTE FUNCTION update_storage_config_updated_at();
