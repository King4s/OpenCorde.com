-- Migration 080: 2FA recovery codes
-- One-time backup codes for TOTP lockout prevention.
-- Codes are stored Argon2id-hashed; plain codes shown once at generation.

CREATE TABLE totp_recovery_codes (
    id              BIGINT PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_hash       TEXT NOT NULL,
    used_at         TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_recovery_codes_user ON totp_recovery_codes (user_id);
CREATE INDEX idx_recovery_codes_active ON totp_recovery_codes (user_id)
    WHERE used_at IS NULL;
