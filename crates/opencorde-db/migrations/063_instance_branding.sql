-- Migration 063: Instance branding
-- Adds branding customization columns to instance_setup:
-- tagline, primary_color, logo_url, favicon_url, custom_css
ALTER TABLE instance_setup
  ADD COLUMN IF NOT EXISTS tagline       TEXT,
  ADD COLUMN IF NOT EXISTS primary_color TEXT,
  ADD COLUMN IF NOT EXISTS logo_url      TEXT,
  ADD COLUMN IF NOT EXISTS favicon_url   TEXT,
  ADD COLUMN IF NOT EXISTS custom_css    TEXT;
