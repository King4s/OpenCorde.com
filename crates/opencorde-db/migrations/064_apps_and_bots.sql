-- Migration 064: Applications, bot users, OAuth scopes, and install grants
-- Foundation for apps/bots/slash commands/webhooks (Milestone 7)

-- 1. Create applications table
CREATE TABLE applications (
    id              BIGINT PRIMARY KEY,
    name            VARCHAR(32) NOT NULL,
    description     TEXT,
    icon_url        TEXT,
    owner_user_id   BIGINT NOT NULL REFERENCES users(id),
    flags           BIGINT NOT NULL DEFAULT 0,
    is_public       BOOLEAN NOT NULL DEFAULT FALSE,
    redirect_uris   JSONB NOT NULL DEFAULT '[]',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_applications_name ON applications (name);
CREATE INDEX idx_applications_owner ON applications (owner_user_id);

-- 2. Add bot-related columns to users
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS is_bot         BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS application_id  BIGINT REFERENCES applications(id);

CREATE INDEX idx_users_is_bot ON users (is_bot) WHERE is_bot = TRUE;

-- 3. Create bot_users table (bot user accounts linked to applications)
CREATE TABLE bot_users (
    id              BIGINT PRIMARY KEY REFERENCES users(id),
    application_id  BIGINT NOT NULL REFERENCES applications(id),
    username        VARCHAR(32) NOT NULL,
    avatar_url      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_bot_users_app ON bot_users (application_id);

-- 4. Create oauth_scopes table (read-only enum/seed)
CREATE TABLE oauth_scopes (
    id          SMALLINT PRIMARY KEY,
    name        VARCHAR(64) NOT NULL UNIQUE,
    description TEXT
);

-- Seed the standard Discord-compatible scopes
INSERT INTO oauth_scopes (id, name, description) VALUES
    (1, 'identify',      'Read your username, avatar, and public key'),
    (2, 'guilds',        'Know what servers you are in'),
    (3, 'bot',           'Add a bot user to a server'),
    (4, 'messages.read', 'Read messages in channels the bot can see'),
    (5, 'applications.commands', 'Create and manage slash commands'),
    (6, 'webhook.incoming', 'Create webhooks in the server')
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name,
    description = EXCLUDED.description;

-- 5. Create app_installs table (grants: which server a bot is installed to)
CREATE TABLE app_installs (
    id              BIGINT PRIMARY KEY,
    application_id  BIGINT NOT NULL REFERENCES applications(id),
    server_id       BIGINT NOT NULL REFERENCES servers(id),
    installed_by    BIGINT NOT NULL REFERENCES users(id),
    scopes          TEXT[] NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_app_installs_app ON app_installs (application_id);
CREATE INDEX idx_app_installs_server ON app_installs (server_id);
CREATE UNIQUE INDEX idx_app_installs_app_server ON app_installs (application_id, server_id);
