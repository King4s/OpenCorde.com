-- Migration 057: Instance setup tracking
-- Tracks whether the initial instance setup wizard has been completed.
-- Only one row should ever exist; created by the setup endpoint on completion.
CREATE TABLE IF NOT EXISTS instance_setup (
    id              BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id = TRUE),
    setup_completed BOOLEAN NOT NULL DEFAULT FALSE,
    instance_name   TEXT,
    base_url        TEXT,
    admin_user_id   BIGINT REFERENCES users(id) ON DELETE SET NULL,
    completed_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Ensure only one row exists
CREATE UNIQUE INDEX IF NOT EXISTS idx_instance_setup_singleton ON instance_setup ((TRUE));
