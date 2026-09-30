-- Migration 081: User OAuth authorizations
-- Tracks which users have authorized which OAuth2 applications.
-- Populated when a user approves an authorization request.
-- Queryable by the authorized-apps API and revocable per-app.

CREATE TABLE user_oauth_authorizations (
    id              BIGINT PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    application_id  BIGINT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    scope           VARCHAR(512) NOT NULL,
    authorized_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_oauth_auth_user ON user_oauth_authorizations (user_id);
CREATE UNIQUE INDEX idx_user_oauth_auth_user_app ON user_oauth_authorizations (user_id, application_id);
