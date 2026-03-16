# OpenCorde Task Tracker

Organized by phase and week. Check items off as they complete.

**Split files** (to stay under 300 lines per file):
- **This file**: Week 0 (done) + Weeks 1-2 (done)
- [tasks-weeks-3-8.md](./tasks-weeks-3-8.md): Auth, Servers, Messaging
- [tasks-weeks-9-12.md](./tasks-weeks-9-12.md): Voice, Desktop Client
- [tasks-future.md](./tasks-future.md): Phase 2-3

---

## Phase 1: MVP (Weeks 0-12)

### Week 0: AI Scaffolding

**Status:** COMPLETE

- [x] Create ReadMeFirst.md
- [x] Create project-map.yaml
- [x] Create decisions.md
- [x] Create all INDEX.md files (in each crate/module as created)
- [x] Create tasks.md
- [x] Create quality-gate.md
- [x] Create CLAUDE.md, .cursorrules, .github/copilot-instructions.md
- [x] Create Cargo.toml workspace
- [x] Create docker-compose.yml
- [x] Create .env.example
- [x] Create LICENSE (AGPL-3.0)
- [x] Create .gitignore
- [x] Initialize git repository
- [x] Verify `cargo check --workspace` passes
- [x] Verify `cargo clippy --workspace -- -D warnings` passes
- [x] Verify `cargo fmt --check` passes

---

### Weeks 1-2: Foundation

**Status:** COMPLETE

**Objective:** Core types, ID generation, permission system, database schema.

#### opencorde-core Crate

- [x] Implement Snowflake ID generator (snowflake.rs — 270 lines)
  - Custom epoch 2024-01-01, 64-bit [42:timestamp][5:worker][5:process][12:sequence]
  - SnowflakeGenerator, Snowflake newtype, Display/FromStr, serde as string
  - 11 unit tests (monotonicity, uniqueness, conversions, timestamp extraction)
- [x] Implement permission bitfield system (permissions.rs + permission_compute.rs)
  - 27 permission flags via bitflags crate (general, text, voice, admin)
  - PermissionOverwrite, OverwriteType, compute_permissions()
  - Admin bypass, role/member overwrite precedence
  - 9 unit tests
- [x] Create core model types (8 model files in models/)
  - user.rs, server.rs, channel.rs, message.rs, member.rs, role.rs, invite.rs, voice_state.rs
  - All Serialize/Deserialize/Debug/Clone, comprehensive tests
  - 41 model tests
- [x] Create shared event types (gateway.rs + events.rs)
  - 19 GatewayEvent variants (lifecycle, messages, typing, presence, voice, servers, channels, members)
  - Serde tagged JSON: {"type": "EventName", "data": {...}}
  - 8 serialization tests

#### opencorde-db Crate

- [x] Create database connection pool (lib.rs — create_pool, run_migrations, health_check)
- [x] Create 9 SQL migrations
  - 001_users, 002_servers, 003_channels, 004_messages, 005_roles
  - 006_server_members + member_roles, 007_invites, 008_voice_states, 009_files
  - Proper indexes, foreign keys, cascading deletes
- [x] Implement user repository (user_repo.rs — full CRUD)
- [x] Implement server repository (server_repo.rs — full CRUD + list_by_user)
- [x] Implement channel repository (channel_repo.rs — full CRUD + list_by_server)
- [x] Implement message repository (message_repo.rs — full CRUD + cursor pagination)
- [x] Implement member repository (member_repo.rs — membership + role assignment)

#### opencorde-api Crate (Setup Only)

- [x] Create Axum skeleton with health check (GET /api/v1/health)
  - main.rs: tokio::main, config loading, DB pool, migration, router
- [x] Add request ID middleware (UUID per request via MakeRequestUuid)
- [x] Add CORS middleware (strict prod, permissive dev)
- [x] Create error handling (ApiError enum + IntoResponse, 7 variants)
- [x] Set up tracing (JSON in prod, pretty in dev, env filter from RUST_LOG)
- [x] Config from env vars with defaults and secret masking

#### Quality Gate

- [x] `cargo check --workspace` — passes
- [x] `cargo clippy --workspace -- -D warnings` — 0 warnings
- [x] `cargo fmt --check` — passes
- [x] `cargo test --workspace` — 99 tests passing (76 core + 8 db + 15 api)
- [x] No file exceeds 300 lines (max: 275)
- [x] All files have structured doc headers
- [x] INDEX.md files updated

#### Infrastructure (remaining)

- [ ] Verify docker-compose services start on dev server
- [ ] Install Rust toolchain on dev server (192.168.140.140)
- [ ] Install pnpm on dev server

---

## Tracking Notes

- Each task must pass quality-gate.md before completion
- Update ReadMeFirst.md weekly with progress
- Blockers noted inline with `BLOCKED: reason`

**Last updated:** 2026-03-16
