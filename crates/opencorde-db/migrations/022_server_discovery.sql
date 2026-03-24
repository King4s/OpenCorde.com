-- Migration: 022_server_discovery
-- Adds public discovery listing for servers.

ALTER TABLE servers ADD COLUMN public BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE servers ADD COLUMN member_count INT NOT NULL DEFAULT 1;
ALTER TABLE servers ADD COLUMN tags VARCHAR(200);
