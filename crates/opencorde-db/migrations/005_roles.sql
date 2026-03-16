-- Create roles table
-- permissions is a 64-bit bitfield stored as BIGINT

CREATE TABLE roles (
    id          BIGINT PRIMARY KEY,
    server_id   BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name        VARCHAR(100) NOT NULL,
    permissions BIGINT NOT NULL DEFAULT 0,
    color       INT,
    position    INT NOT NULL DEFAULT 0,
    mentionable BOOLEAN NOT NULL DEFAULT false,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_roles_server ON roles (server_id);
