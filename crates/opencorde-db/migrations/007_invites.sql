-- Create invites table

CREATE TABLE invites (
    code        VARCHAR(16) PRIMARY KEY,
    server_id   BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    creator_id  BIGINT NOT NULL REFERENCES users(id),
    uses        INT NOT NULL DEFAULT 0,
    max_uses    INT,           -- NULL = unlimited
    expires_at  TIMESTAMPTZ,   -- NULL = never expires
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_invites_server ON invites (server_id);
