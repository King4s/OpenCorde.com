-- Migration 041: Refresh token JTI table for rotation and theft detection
--
-- Each issued refresh token has a unique JTI (UUID v4) stored here.
-- On refresh: if the JTI is already revoked, ALL user tokens are revoked (theft detected).
-- On successful refresh: old JTI is revoked, new JTI is inserted.

CREATE TABLE refresh_tokens (
    jti         TEXT        PRIMARY KEY,
    user_id     BIGINT      NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ NOT NULL,
    revoked     BOOL        NOT NULL DEFAULT FALSE
);

-- Fast lookup when enforcing a refresh (primary path)
CREATE INDEX refresh_tokens_user_id_idx   ON refresh_tokens (user_id);
-- Fast cleanup of expired rows
CREATE INDEX refresh_tokens_expires_at_idx ON refresh_tokens (expires_at);
