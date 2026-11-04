-- Add hoist, position, and icon_url columns to roles table.
-- position already exists in some deployments (added in 005_roles.sql);
-- IF NOT EXISTS makes this idempotent.
ALTER TABLE roles
    ADD COLUMN IF NOT EXISTS hoist BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS position INT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS icon_url TEXT;
