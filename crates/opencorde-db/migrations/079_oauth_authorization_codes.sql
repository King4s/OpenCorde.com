-- Migration 079: OAuth2 authorization codes (one-time use)
-- Issued during the authorize flow, consumed at token exchange.
-- Codes are stored hashed (Argon2id) — only the user sees plaintext once.

CREATE TABLE oauth_authorization_codes (
    id              BIGINT PRIMARY KEY,
    application_id  BIGINT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_hash       TEXT NOT NULL,
    redirect_uri    VARCHAR(2048) NOT NULL,
    scope           VARCHAR(512) NOT NULL,
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    used_at         TIMESTAMPTZ
);

CREATE INDEX idx_auth_codes_app ON oauth_authorization_codes (application_id);
-- NOW() is not IMMUTABLE and thus not allowed in index predicates;
-- expiry is filtered at query time (expires_at > NOW() in WHERE clauses).
CREATE INDEX idx_auth_codes_active ON oauth_authorization_codes (application_id)
    WHERE used_at IS NULL;
