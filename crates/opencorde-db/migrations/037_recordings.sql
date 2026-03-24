-- Migration: 037_recordings
-- Creates the recordings table for LiveKit Egress-based call recording.
-- Each row tracks one Egress job (start → stop lifecycle).

CREATE TABLE IF NOT EXISTS recordings (
    id              BIGSERIAL PRIMARY KEY,
    server_id       BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    channel_id      BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    started_by      BIGINT NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    egress_id       TEXT NOT NULL UNIQUE,  -- LiveKit Egress job ID
    status          TEXT NOT NULL DEFAULT 'recording',  -- recording | stopped | failed
    file_path       TEXT,  -- MinIO object key when complete
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    stopped_at      TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_recordings_channel ON recordings(channel_id);
