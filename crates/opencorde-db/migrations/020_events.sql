-- Migration: 020_events
-- Scheduled server events with RSVP tracking.

CREATE TYPE event_status AS ENUM ('scheduled', 'active', 'completed', 'cancelled');
CREATE TYPE event_location_type AS ENUM ('voice', 'external', 'stage');

CREATE TABLE server_events (
    id              BIGINT PRIMARY KEY,
    server_id       BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    channel_id      BIGINT REFERENCES channels(id) ON DELETE SET NULL,
    creator_id      BIGINT NOT NULL REFERENCES users(id),
    title           VARCHAR(100) NOT NULL,
    description     TEXT,
    location_type   event_location_type NOT NULL DEFAULT 'external',
    location_name   VARCHAR(200),
    starts_at       TIMESTAMPTZ NOT NULL,
    ends_at         TIMESTAMPTZ,
    status          event_status NOT NULL DEFAULT 'scheduled',
    cover_image_url VARCHAR(500),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE event_rsvps (
    event_id    BIGINT NOT NULL REFERENCES server_events(id) ON DELETE CASCADE,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rsvp_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (event_id, user_id)
);

CREATE INDEX idx_events_server ON server_events(server_id, starts_at ASC);
CREATE INDEX idx_events_status ON server_events(status, starts_at ASC);
CREATE INDEX idx_rsvps_user ON event_rsvps(user_id);
