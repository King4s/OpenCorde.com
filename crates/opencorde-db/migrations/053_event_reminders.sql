-- Migration: 053_event_reminders
-- Automatic event lifecycle transitions and user notification preferences.

-- Track when a reminder was sent so we don't spam.
ALTER TABLE server_events ADD COLUMN IF NOT EXISTS reminded_at TIMESTAMPTZ;

-- User-level opt-in for event reminders (default TRUE).
CREATE TABLE IF NOT EXISTS user_notification_settings (
    user_id                   BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    event_reminders_enabled   BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at                TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for the background task: find scheduled events starting soon that haven't been reminded.
CREATE INDEX IF NOT EXISTS idx_events_reminder ON server_events (status, starts_at) WHERE reminded_at IS NULL;

-- Index for the background task: find active events that have ended.
CREATE INDEX IF NOT EXISTS idx_events_ended ON server_events (status, ends_at) WHERE ends_at IS NOT NULL;
