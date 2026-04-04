-- Migration 049: Server onboarding configuration
CREATE TABLE IF NOT EXISTS server_onboarding (
    server_id       BIGINT PRIMARY KEY REFERENCES servers(id) ON DELETE CASCADE,
    enabled         BOOLEAN NOT NULL DEFAULT FALSE,
    welcome_message TEXT,
    prompts         JSONB NOT NULL DEFAULT '[]',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
