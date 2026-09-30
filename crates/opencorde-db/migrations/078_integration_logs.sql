-- Migration 078: Integration logs — track bot/app/webhook activity

CREATE TABLE integration_logs (
    id              BIGINT PRIMARY KEY,
    server_id       BIGINT REFERENCES servers(id) ON DELETE CASCADE,
    application_id  BIGINT REFERENCES applications(id) ON DELETE SET NULL,
    actor_bot_user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    action_type     VARCHAR(64) NOT NULL,
    status          VARCHAR(24) NOT NULL DEFAULT 'success',
    metadata        JSONB NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_integration_logs_server ON integration_logs (server_id);
CREATE INDEX idx_integration_logs_app ON integration_logs (application_id);
CREATE INDEX idx_integration_logs_created ON integration_logs (created_at DESC);
