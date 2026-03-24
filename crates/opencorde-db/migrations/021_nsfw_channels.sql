-- Migration: 021_nsfw_channels
-- Adds NSFW flag to channels for content moderation.

ALTER TABLE channels ADD COLUMN nsfw BOOLEAN NOT NULL DEFAULT FALSE;
