-- Migration 077: Message embeds and components (Discord-compatible rich content)
-- Embeds: title, description, color, fields, images, thumbnails, footer, author

CREATE TABLE message_embeds (
    id          BIGINT PRIMARY KEY,
    message_id  BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    position    SMALLINT NOT NULL DEFAULT 0,
    payload     JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_message_embeds_msg ON message_embeds (message_id);

CREATE TABLE message_components (
    id          BIGINT PRIMARY KEY,
    message_id  BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    payload     JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_message_components_msg ON message_components (message_id);
