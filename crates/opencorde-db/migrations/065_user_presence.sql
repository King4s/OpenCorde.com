-- Migration 065: User presence system
-- Extends the users table with custom status and activity fields
-- for full Discord-parity presence.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS custom_status_text VARCHAR(128),
    ADD COLUMN IF NOT EXISTS custom_status_expires_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS activity_type VARCHAR(32),
    ADD COLUMN IF NOT EXISTS activity_name VARCHAR(128);
