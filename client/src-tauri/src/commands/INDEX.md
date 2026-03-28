# /client/src-tauri/src/commands/

Purpose: Tauri IPC command handlers for desktop app.

Pattern: One file per domain. Commands are invoked from SvelteKit frontend via Tauri invoke().

## Files

| File | Purpose | Commands |
|------|---------|----------|
| auth.rs | OS keychain credential storage | `store_token`, `get_token`, `delete_token` |
| settings.rs | App preferences and platform detection | `get_settings`, `save_settings`, `get_platform` |
| crypto.rs | Client-side E2EE stubs (future) | `encrypt_message`, `decrypt_message` |
| mod.rs | Command registration and exports | Module index |

## Architecture

Each command module exports `#[tauri::command]` functions that are:
1. Defined with `#[tauri::command]` attribute
2. Listed in `lib.rs` under `tauri::generate_handler!` macro
3. Invoked from SvelteKit frontend via `invoke('command_name')`

Modules use `tauri::AppHandle` to access app context (file paths, plugins, etc.)
