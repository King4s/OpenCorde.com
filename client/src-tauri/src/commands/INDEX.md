# /client/src-tauri/src/commands/

Purpose: Tauri IPC command handlers for desktop app.

Pattern: One file per domain. Commands are invoked from SvelteKit frontend via Tauri invoke().

| File | Purpose |
|------|---------|
| auth.rs | Login, logout, token refresh, credential storage |
| crypto.rs | Local encryption/decryption (client-side E2EE) |
| settings.rs | App preferences, theme, audio device selection |
| mod.rs | Command registration |
