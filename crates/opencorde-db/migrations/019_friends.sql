-- Migration: 019_friends
-- Friend requests, friendships, and user blocks.

CREATE TYPE relationship_status AS ENUM ('pending', 'accepted', 'blocked');

CREATE TABLE relationships (
    id          BIGINT PRIMARY KEY,
    from_user   BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    to_user     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status      relationship_status NOT NULL DEFAULT 'pending',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (from_user, to_user)
);

CREATE INDEX idx_relationships_from ON relationships(from_user, status);
CREATE INDEX idx_relationships_to ON relationships(to_user, status);
