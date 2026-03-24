-- Migration 038: Push notification device token storage
--
-- Stores per-user push tokens for Web Push (browser), FCM (Android), and APNS (iOS).
-- UNIQUE(user_id, token) prevents duplicate registrations from the same device.

CREATE TABLE IF NOT EXISTS push_tokens (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token       TEXT NOT NULL,
    -- 'web' = Web Push (browser), 'fcm' = Firebase Cloud Messaging (Android),
    -- 'apns' = Apple Push Notification Service (iOS)
    platform    TEXT NOT NULL CHECK (platform IN ('web', 'fcm', 'apns')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, token)
);

CREATE INDEX IF NOT EXISTS idx_push_tokens_user ON push_tokens(user_id);
