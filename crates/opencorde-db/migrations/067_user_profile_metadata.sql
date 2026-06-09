-- Migration 067: User profile metadata
-- Adds pronouns, display_name, and banner_url to user profiles

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS pronouns VARCHAR(32),
    ADD COLUMN IF NOT EXISTS display_name VARCHAR(64),
    ADD COLUMN IF NOT EXISTS banner_url TEXT;
