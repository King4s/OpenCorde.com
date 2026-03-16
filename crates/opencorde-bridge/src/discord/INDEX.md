# /crates/opencorde-bridge/src/discord/

Purpose: Discord bridge implementation.

Pattern: Separated by concern — gateway connection, REST API, message mapping, ghost user management.

| File | Purpose |
|------|---------|
| gateway.rs | Discord Gateway connection, heartbeat, event recv |
| api.rs | Discord REST API client (send messages, manage channels, etc.) |
| mapper.rs | Message/event transformation between Discord and OpenCorde formats |
| puppet.rs | Ghost user lifecycle — create Discord webhooks for bridged users |
| mod.rs | Module exports |
