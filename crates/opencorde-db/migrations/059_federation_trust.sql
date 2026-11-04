-- Federation trust and moderation settings
-- Adds trust levels to mesh peers and instance-level federation policy

-- Trust levels for peer servers
ALTER TABLE mesh_peers ADD COLUMN trust_level SMALLINT NOT NULL DEFAULT 1;
-- 0 = untrusted, 1 = low, 2 = medium, 3 = high

-- Instance federation settings
CREATE TABLE federation_settings (
    id              SMALLINT PRIMARY KEY DEFAULT 1 CHECK (id = 1), -- singleton
    allow_remote_dms BOOLEAN NOT NULL DEFAULT true,
    dm_policy        SMALLINT NOT NULL DEFAULT 0, -- 0 = anyone, 1 = mutuals, 2 = none
    forward_reports  BOOLEAN NOT NULL DEFAULT true,
    instance_blocklist TEXT NOT NULL DEFAULT ''
);

INSERT INTO federation_settings (id, allow_remote_dms, dm_policy, forward_reports, instance_blocklist)
VALUES (1, true, 0, true, '');
