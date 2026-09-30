-- Migration 076: Application commands (Discord-compatible)
-- App-owned commands with options, types, and version tracking

CREATE TABLE application_commands (
    id              BIGINT PRIMARY KEY,
    application_id  BIGINT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    server_id       BIGINT REFERENCES servers(id) ON DELETE CASCADE,
    name            VARCHAR(32) NOT NULL,
    description     VARCHAR(100) NOT NULL DEFAULT '',
    command_type    SMALLINT NOT NULL DEFAULT 1,  -- 1=CHAT_INPUT, 2=USER, 3=MESSAGE
    options         JSONB NOT NULL DEFAULT '[]',
    default_member_permissions BIGINT,
    dm_permission   BOOLEAN NOT NULL DEFAULT FALSE,
    version         BIGINT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_app_commands_name ON application_commands (application_id, server_id, name);
CREATE INDEX idx_app_commands_app ON application_commands (application_id);
CREATE INDEX idx_app_commands_server ON application_commands (server_id);

-- Interaction table: one-time tokens for command/component/modal invocations
CREATE TABLE interactions (
    id              BIGINT PRIMARY KEY,
    application_id  BIGINT NOT NULL REFERENCES applications(id),
    token_hash      TEXT NOT NULL,
    interaction_type SMALLINT NOT NULL,  -- 2=command, 3=component, 4=modal, 5=autocomplete
    command_id      BIGINT REFERENCES application_commands(id),
    server_id       BIGINT REFERENCES servers(id),
    channel_id      BIGINT REFERENCES channels(id),
    user_id         BIGINT NOT NULL REFERENCES users(id),
    message_id      BIGINT,
    data            JSONB NOT NULL DEFAULT '{}',
    response_state  VARCHAR(24) NOT NULL DEFAULT 'pending',
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    responded_at    TIMESTAMPTZ
);

CREATE INDEX idx_interactions_token ON interactions (token_hash);
CREATE INDEX idx_interactions_expires ON interactions (expires_at) WHERE response_state = 'pending';
