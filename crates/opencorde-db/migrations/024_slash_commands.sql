CREATE TABLE slash_commands (
    id BIGINT PRIMARY KEY,
    server_id BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name VARCHAR(32) NOT NULL,
    description VARCHAR(100) NOT NULL DEFAULT '',
    handler_url VARCHAR(500) NOT NULL,
    created_by BIGINT NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(server_id, name)
);

CREATE INDEX idx_slash_commands_server ON slash_commands(server_id);
