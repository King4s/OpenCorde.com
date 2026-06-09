-- Migration: 071_friend_nicknames
-- Adds friend nickname support and blocked-users index for listing.

ALTER TABLE relationships ADD COLUMN IF NOT EXISTS nickname VARCHAR(100);
CREATE INDEX IF NOT EXISTS idx_relationships_blocked ON relationships(from_user) WHERE status = 'blocked';
