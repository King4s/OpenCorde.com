-- Create files table for uploaded attachments

CREATE TABLE files (
    id           BIGINT PRIMARY KEY,
    uploader_id  BIGINT NOT NULL REFERENCES users(id),
    filename     VARCHAR(255) NOT NULL,
    bucket_key   TEXT NOT NULL,
    size         BIGINT NOT NULL,
    content_type VARCHAR(127) NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_files_uploader ON files (uploader_id);
