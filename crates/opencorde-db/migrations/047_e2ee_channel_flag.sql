-- Migration 047: Add e2ee_enabled flag to channels
-- Marks a channel as end-to-end encrypted (OpenMLS group managed by clients).
ALTER TABLE channels ADD COLUMN IF NOT EXISTS e2ee_enabled BOOL NOT NULL DEFAULT false;
