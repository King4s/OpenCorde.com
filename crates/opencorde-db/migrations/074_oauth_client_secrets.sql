-- Migration 074: OAuth2 client secrets with Argon2id hashing
-- Store secrets hashed; plain secret returned once at creation

CREATE TABLE oauth_client_secrets (
    id              BIGINT PRIMARY KEY,
    application_id  BIGINT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    secret_prefix   VARCHAR(12) NOT NULL,
    secret_hash     TEXT NOT NULL,
    created_by      BIGINT NOT NULL REFERENCES users(id),
    label           VARCHAR(80),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at      TIMESTAMPTZ
);

CREATE INDEX idx_client_secrets_app ON oauth_client_secrets (application_id);
CREATE INDEX idx_client_secrets_active ON oauth_client_secrets (application_id) WHERE revoked_at IS NULL;
