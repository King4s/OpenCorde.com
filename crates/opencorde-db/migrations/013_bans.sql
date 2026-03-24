-- Server ban list
CREATE TABLE server_bans (
    server_id   BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    banned_by   BIGINT NOT NULL REFERENCES users(id),
    reason      TEXT,
    banned_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (server_id, user_id)
);

-- User timeouts (temporary mutes)
CREATE TABLE member_timeouts (
    server_id       BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    timeout_until   TIMESTAMPTZ NOT NULL,
    reason          TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (server_id, user_id)
);
