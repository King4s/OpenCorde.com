-- Migration 048: Soundboard sounds per server
CREATE TABLE IF NOT EXISTS soundboard_sounds (
    id          BIGINT PRIMARY KEY,
    server_id   BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name        VARCHAR(32) NOT NULL,
    file_key    TEXT NOT NULL,
    uploader_id BIGINT NOT NULL REFERENCES users(id),
    volume      SMALLINT NOT NULL DEFAULT 100,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (server_id, name)
);

CREATE INDEX IF NOT EXISTS idx_soundboard_server ON soundboard_sounds(server_id);
