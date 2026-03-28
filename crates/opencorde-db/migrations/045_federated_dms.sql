-- Migration 045: Federated DM support
-- Allows DM channels to span two different OpenCorde servers.
-- The remote participant is tracked by address ("username@hostname") rather than
-- a local user account, since they live on another server.

-- Track the remote peer on DM channels that cross server boundaries
ALTER TABLE dm_channels
    ADD COLUMN IF NOT EXISTS remote_peer_address TEXT,
    ADD COLUMN IF NOT EXISTS remote_server TEXT;

COMMENT ON COLUMN dm_channels.remote_peer_address IS
    'For federated DMs: "username@hostname" of the remote participant';
COMMENT ON COLUMN dm_channels.remote_server IS
    'For federated DMs: hostname of the remote server (used to route outbound messages)';

-- Allow federated messages where the author is on a remote server.
-- When federated_author is set, author_id is NULL and the display name
-- comes from the remote server instead.
ALTER TABLE dm_messages
    ADD COLUMN IF NOT EXISTS federated_author TEXT;

ALTER TABLE dm_messages
    ALTER COLUMN author_id DROP NOT NULL;

COMMENT ON COLUMN dm_messages.federated_author IS
    'For messages received from a remote server: "username@hostname" display name';

CREATE INDEX IF NOT EXISTS idx_dm_channels_remote_server
    ON dm_channels (remote_server) WHERE remote_server IS NOT NULL;
