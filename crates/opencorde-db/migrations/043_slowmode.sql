-- Migration: 043_slowmode
-- Adds slowmode_delay column to channels.
-- slowmode_delay: seconds a user must wait between messages (0 = disabled).

ALTER TABLE channels ADD COLUMN IF NOT EXISTS slowmode_delay INT NOT NULL DEFAULT 0;
