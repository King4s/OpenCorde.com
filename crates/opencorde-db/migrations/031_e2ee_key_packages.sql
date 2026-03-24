-- Migration 031: E2EE key packages
-- One-time-use cryptographic bundles uploaded by clients.
-- When a user is added to an E2EE group, one of their unused key packages is consumed.

CREATE TABLE IF NOT EXISTS e2ee_key_packages (
    id         BIGSERIAL    PRIMARY KEY,
    user_id    BIGINT       NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_package BYTEA       NOT NULL,           -- TLS-serialized OpenMLS KeyPackage
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    consumed_at TIMESTAMPTZ                     -- NULL = available, set when used to add to group
);

-- Only index unconsumed packages (most queries filter on consumed_at IS NULL)
CREATE INDEX IF NOT EXISTS idx_e2ee_key_packages_user_unconsumed
    ON e2ee_key_packages(user_id)
    WHERE consumed_at IS NULL;
