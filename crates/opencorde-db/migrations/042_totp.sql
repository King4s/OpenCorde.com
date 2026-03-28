-- Migration 042: TOTP two-factor authentication
--
-- Adds TOTP (RFC 6238, Google Authenticator-compatible) support to user accounts.
-- totp_secret is stored as base32 (RFC 4648) and only present once the user
-- initiates setup (not necessarily enabled yet).
-- totp_enabled is set TRUE only after the user successfully verifies a code.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS totp_secret  TEXT,
    ADD COLUMN IF NOT EXISTS totp_enabled BOOL NOT NULL DEFAULT FALSE;
