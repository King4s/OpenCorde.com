-- Add Steam ID to users for Steam OpenID login
ALTER TABLE users ADD COLUMN IF NOT EXISTS steam_id VARCHAR(20) UNIQUE;
CREATE INDEX IF NOT EXISTS idx_users_steam_id ON users(steam_id) WHERE steam_id IS NOT NULL;
