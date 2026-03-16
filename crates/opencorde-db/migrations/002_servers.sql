-- Create servers table

CREATE TABLE servers (
    id          BIGINT PRIMARY KEY,
    name        VARCHAR(100) NOT NULL,
    owner_id    BIGINT NOT NULL REFERENCES users(id),
    icon_url    TEXT,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_servers_owner ON servers (owner_id);
