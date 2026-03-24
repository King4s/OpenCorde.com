-- Migration 033: Email Verification
-- Adds email_verified flag and verification token columns to users table.
-- Token is cleared after successful verification.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS email_verified            BOOLEAN      NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS email_verification_token  TEXT,
    ADD COLUMN IF NOT EXISTS email_verification_expires_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_users_verification_token
    ON users (email_verification_token)
    WHERE email_verification_token IS NOT NULL;
