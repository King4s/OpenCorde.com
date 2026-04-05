# /client/

Purpose: Tauri 2.0 desktop app with SvelteKit frontend.

Pattern: src-tauri/ for Rust backend, src/ for SvelteKit frontend. Shared IPC protocol.

| Directory     | Purpose                                                   |
| ------------- | --------------------------------------------------------- |
| src-tauri/src | Tauri Rust commands (auth, crypto, settings)              |
| src/lib       | Shared SvelteKit library (API client, stores, components) |
| src/routes    | SvelteKit file-based routes                               |
| src-tauri     | Cargo crate for Tauri                                     |
