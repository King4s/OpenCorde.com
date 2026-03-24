-- Password Reset Tokens Table
-- Stores one-time use tokens for initiating password resets.
--
-- Features:
-- - 64-character hex tokens (32 bytes encoded)
-- - Automatic expiry after 1 hour
-- - One-time use tracking via used_at column
-- - Cascade delete on user deletion
--
-- Depends On:
-- - users table (foreign key)

CREATE TABLE IF NOT EXISTS password_reset_tokens (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token VARCHAR(64) NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_prt_token ON password_reset_tokens(token) WHERE used_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_prt_user ON password_reset_tokens(user_id);
