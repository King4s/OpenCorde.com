# /crates/

Purpose: Rust crates in the Cargo workspace.

Pattern: Each subdirectory is a standalone crate with its own Cargo.toml.

| Crate | Type | Purpose | Entry |
|-------|------|---------|-------|
| opencorde-api | Binary | REST API + WebSocket gateway | src/main.rs |
| opencorde-core | Library | Shared domain types, models, permissions | src/lib.rs |
| opencorde-db | Library | PostgreSQL database layer via sqlx | src/lib.rs |
| opencorde-crypto | Library | E2EE layer using OpenMLS (Phase 2) | src/lib.rs |
| opencorde-bridge | Binary | External platform integrations | src/main.rs |
| opencorde-search | Library | Full-text search using Tantivy | src/lib.rs |
