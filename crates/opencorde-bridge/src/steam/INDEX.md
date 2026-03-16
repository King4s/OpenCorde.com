# /crates/opencorde-bridge/src/steam/

Purpose: Steam integration (OAuth + friends).

Pattern: Separated by auth flow and social features.

| File | Purpose |
|------|---------|
| oauth.rs | Steam OpenID authentication flow |
| friends.rs | Steam friends list sync, presence polling |
| mod.rs | Module exports |
