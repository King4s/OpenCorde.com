-- Create server_members join table and member_roles

CREATE TABLE server_members (
    user_id     BIGINT NOT NULL REFERENCES users(id),
    server_id   BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    nickname    VARCHAR(32),
    joined_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, server_id)
);

CREATE INDEX idx_members_server ON server_members (server_id);

CREATE TABLE member_roles (
    user_id     BIGINT NOT NULL,
    server_id   BIGINT NOT NULL,
    role_id     BIGINT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, server_id, role_id),
    FOREIGN KEY (user_id, server_id) REFERENCES server_members(user_id, server_id) ON DELETE CASCADE
);
