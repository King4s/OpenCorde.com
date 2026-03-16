-- Create users table
-- Ed25519 keypair identity with optional email/password for recovery
-- Snowflake IDs (BIGINT), timestamps with timezone

CREATE TABLE users (
    id              BIGINT PRIMARY KEY,
    username        VARCHAR(32) NOT NULL,
    public_key      VARCHAR(64) NOT NULL,      -- Ed25519 public key, hex-encoded (64 chars)
    email           VARCHAR(255),              -- Optional: for password recovery
    password_hash   TEXT,                      -- Optional: for email+password login
    avatar_url      TEXT,
    status          SMALLINT NOT NULL DEFAULT 3,  -- 0=Online, 1=Idle, 2=DND, 3=Offline
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_users_username ON users (username);
CREATE UNIQUE INDEX idx_users_public_key ON users (public_key);
CREATE UNIQUE INDEX idx_users_email ON users (email) WHERE email IS NOT NULL;
