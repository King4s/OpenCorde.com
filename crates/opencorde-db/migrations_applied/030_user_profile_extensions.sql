-- Migration 030: User profile extensions
-- Adds bio and custom status message to user profiles

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS bio TEXT,
    ADD COLUMN IF NOT EXISTS status_message VARCHAR(128);
