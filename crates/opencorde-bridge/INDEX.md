# /crates/opencorde-bridge/

Purpose: Bridge service for connecting OpenCorde to external platforms. Binary crate.

Entry: src/main.rs
Pattern: One subdirectory per platform. Each platform has independent configuration and lifecycle.

| Platform | Purpose | Files |
|----------|---------|-------|
| src/discord | Discord gateway + REST sync | gateway.rs, api.rs, mapper.rs, puppet.rs |
| src/steam | Steam OAuth + friends list | oauth.rs, friends.rs |
| src/main.rs | Service startup, platform coordination | — |
