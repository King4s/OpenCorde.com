-- Migration 075: Bot tokens with Argon2id hashing and gateway intents
-- Plain token returned once; server stores only hash + prefix

CREATE TABLE bot_tokens (
    id              BIGINT PRIMARY KEY,
    application_id  BIGINT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    bot_user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_prefix    VARCHAR(16) NOT NULL,
    token_hash      TEXT NOT NULL,
    intents         BIGINT NOT NULL DEFAULT 0,
    label           VARCHAR(80),
    last_used_at    TIMESTAMPTZ,
    created_by      BIGINT NOT NULL REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at      TIMESTAMPTZ
);

CREATE INDEX idx_bot_tokens_app ON bot_tokens (application_id);
CREATE INDEX idx_bot_tokens_active ON bot_tokens (application_id) WHERE revoked_at IS NULL;
