CREATE TABLE audit_log (
    id BIGINT PRIMARY KEY,
    server_id BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    actor_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    actor_username VARCHAR(80),
    action VARCHAR(80) NOT NULL,
    target_id BIGINT,
    target_type VARCHAR(40),
    changes JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_log_server ON audit_log(server_id, created_at DESC);
CREATE INDEX idx_audit_log_actor ON audit_log(actor_id);
