-- Migration: 052_event_recurrence
-- Add recurrence support and calendar export fields to server_events.

ALTER TABLE server_events
    ADD COLUMN recurrence_rule JSONB,
    ADD COLUMN recurrence_end_date TIMESTAMPTZ,
    ADD COLUMN parent_event_id BIGINT REFERENCES server_events(id) ON DELETE CASCADE,
    ADD COLUMN is_recurring BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX idx_events_parent ON server_events(parent_event_id);
CREATE INDEX idx_events_recurring ON server_events(is_recurring, recurrence_end_date);

-- Table to store generated recurring event instances (materialized)
CREATE TABLE event_instances (
    id              BIGINT PRIMARY KEY,
    parent_event_id BIGINT NOT NULL REFERENCES server_events(id) ON DELETE CASCADE,
    server_id       BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    starts_at       TIMESTAMPTZ NOT NULL,
    ends_at         TIMESTAMPTZ,
    status          event_status NOT NULL DEFAULT 'scheduled',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_instances_parent ON event_instances(parent_event_id, starts_at ASC);
CREATE INDEX idx_instances_server ON event_instances(server_id, starts_at ASC);
