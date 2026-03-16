-- Mesh federation: peer server registry
-- Phase 2: server-to-server peering
-- Enables federated chat across independent OpenMesh servers

CREATE TABLE mesh_peers (
    id              BIGINT PRIMARY KEY,         -- Snowflake ID
    hostname        VARCHAR(255) NOT NULL,     -- Peer server hostname (e.g., "mesh.example.com")
    public_key      VARCHAR(64) NOT NULL,      -- Peer server's Ed25519 public key (hex-encoded)
    status          SMALLINT NOT NULL DEFAULT 0, -- 0=pending, 1=active, 2=suspended
    last_seen_at    TIMESTAMPTZ,               -- Last successful connection timestamp
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_peers_hostname ON mesh_peers (hostname);
CREATE UNIQUE INDEX idx_peers_public_key ON mesh_peers (public_key);
CREATE INDEX idx_peers_status ON mesh_peers (status);
