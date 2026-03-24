-- Migration: 023_automod
-- AutoMod keyword filter rules per server.

CREATE TABLE automod_rules (
    id          BIGINT PRIMARY KEY,
    server_id   BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name        VARCHAR(100) NOT NULL DEFAULT 'Keyword Filter',
    keywords    TEXT NOT NULL,   -- comma-separated list of banned words/phrases
    enabled     BOOLEAN NOT NULL DEFAULT TRUE,
    action      VARCHAR(20) NOT NULL DEFAULT 'delete',  -- 'delete' or 'timeout'
    created_by  BIGINT NOT NULL REFERENCES users(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (server_id, name)
);

CREATE INDEX idx_automod_server ON automod_rules(server_id, enabled);
