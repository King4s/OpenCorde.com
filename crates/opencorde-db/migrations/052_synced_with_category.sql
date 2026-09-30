-- Add synced_with_category flag to channels
-- When true, channel overwrites are kept in sync with parent category overwrites

ALTER TABLE channels ADD COLUMN IF NOT EXISTS synced_with_category BOOLEAN NOT NULL DEFAULT false;
