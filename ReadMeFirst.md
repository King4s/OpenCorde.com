# OpenCorde: ReadMeFirst.md

**THE entry point for all AI agents working on this project.**

## Project Purpose

OpenCorde is a **self-hosted Discord alternative** built with Rust (Axum backend), SvelteKit + Tauri 2.0 frontend, PostgreSQL, Redis, MinIO, and LiveKit. It enables organizations to run feature-complete team communication—servers, channels, messaging, voice/video, file sharing, and E2EE—on their own infrastructure with zero proprietary dependency.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Tauri 2.0 Desktop App                 │
│              SvelteKit + Tailwind CSS + LiveKit JS SDK   │
│          (Windows, macOS, Linux — future: iOS, Android)  │
└──────┬──────────────┬────────────────┬──────────────────┘
       │ REST/WS      │ WebRTC         │ E2EE (MLS/WASM)
       ▼              ▼                ▼
┌──────────────┐ ┌──────────┐  ┌──────────────────┐
│ OpenCorde API│ │ LiveKit  │  │  MinIO (S3)      │
│ (Rust/Axum)  │ │ Server   │  │  File Storage    │
│ REST + WS    │ │ (SFU)    │  └──────────────────┘
│ Gateway      │ │ Voice/   │
└──────┬───────┘ │ Video/   │
       │         │ Screen   │
  ┌────┴────┐    └────┬─────┘
  │         │         │
  ▼         ▼         ▼
┌─────┐ ┌───────┐ ┌───────┐   ┌──────────────────┐
│ PG  │ │ Redis │ │ Redis │   │ Bridge Service   │
│ SQL │ │ Cache │ │ PubSub│   │ (Discord, Steam) │
└─────┘ └───────┘ └───────┘   └──────────────────┘
```

## Project Map

- **root/** — Configuration and documentation (you are here)
  - `ReadMeFirst.md` — This file
  - `project-map.yaml` — Machine-readable project structure
  - `decisions.md` — Architectural decisions with dates and rationales
  - `tasks.md` — Full task tracker (Phase 1-3)
  - `quality-gate.md` — CI/CD acceptance criteria
  - `docker-compose.yml` — Local development services
  - `.env.example` — Environment variable template
  - `LICENSE` — AGPL-3.0-or-later
  - `Cargo.toml` — Rust workspace manifest
  - `.cursorrules` — Cursor IDE configuration
  - `CLAUDE.md` — Claude/Aider instructions
  - `.github/copilot-instructions.md` — GitHub Copilot instructions

- **crates/** — Rust backend (Cargo workspace members)
  - `opencorde-core/` — Type definitions, snowflake IDs, permissions, shared models
  - `opencorde-db/` — Database layer (sqlx + migrations)
  - `opencorde-api/` — REST/WebSocket API (Axum gateway and handlers)
  - `opencorde-crypto/` — E2EE layer using OpenMLS (Phase 2)
  - `opencorde-bridge/` — Bridge service for Discord, Steam integration
  - `opencorde-search/` — Full-text search using Tantivy

- **client/** — Desktop + web frontends
  - `src-tauri/` — Tauri 2.0 Rust backend
  - `src/` — SvelteKit application (routes, pages, components, stores)
  - `package.json` — Node dependencies

- **deploy/** — Deployment configs (LiveKit, Caddy, scripts)
- **docs/** — Auto-generated reference docs (OpenAPI, schema, error codes)
- **tests/** — Integration (testcontainers-rs) and E2E (Playwright) tests

- **INDEX.md files** — Placed in each crate/module directory for AI navigation

## File Index

### opencorde-core (15 files, 76 tests)
- `snowflake.rs` — 64-bit time-ordered ID generator (270 lines)
- `permissions.rs` — Permission bitflags (27 flags) (181 lines)
- `permission_compute.rs` — Overwrite computation logic (141 lines)
- `gateway.rs` — GatewayEvent enum definition (89 lines)
- `events.rs` — Event serialization tests (231 lines)
- `models/user.rs` — User, UserProfile, UserStatus (152 lines)
- `models/server.rs` — Server struct (82 lines)
- `models/channel.rs` — Channel, ChannelType (148 lines)
- `models/message.rs` — Message, Attachment (182 lines)
- `models/member.rs` — Member (server membership) (102 lines)
- `models/role.rs` — Role with permissions (159 lines)
- `models/invite.rs` — Invite with expiry/uses (198 lines)
- `models/voice_state.rs` — VoiceState (132 lines)

### opencorde-db (6 files + 9 migrations, 8 tests)
- `lib.rs` — Pool creation, migrations, health check (72 lines)
- `repos/user_repo.rs` — User CRUD (172 lines)
- `repos/server_repo.rs` — Server CRUD + list_by_user (172 lines)
- `repos/channel_repo.rs` — Channel CRUD + list_by_server (195 lines)
- `repos/message_repo.rs` — Message CRUD + cursor pagination (229 lines)
- `repos/member_repo.rs` — Membership + role assignment (275 lines)
- 9 SQL migrations: users, servers, channels, messages, roles, members, invites, voice_states, files

### opencorde-api (9 files, 15 tests)
- `main.rs` — Server entry point with tracing init (143 lines)
- `lib.rs` — AppState, module exports (55 lines)
- `config.rs` — Env var configuration (186 lines)
- `error.rs` — ApiError + IntoResponse (177 lines)
- `routes/health.rs` — GET /api/v1/health (90 lines)
- `middleware/request_id.rs` — UUID per request (57 lines)
- `middleware/cors.rs` — CORS (strict prod, permissive dev) (67 lines)

## Current Project State

**Phases 1–3 + Sprint 0 Security: COMPLETE. 26/26 browser tests passing. 42 migrations applied.**

All features implemented and deployed at opencorde.com:
- Auth (register/login/JWT/refresh/password-reset UI), user profiles, settings, avatar upload
- Servers, channels (text/voice/stage/forum), messages, reactions, replies, threads, pins
- File attachments (MinIO), WebSocket gateway (real-time messages, typing, presence, unread)
- LiveKit voice/video (VoicePanel, StagePanel), screen share
- DMs, friends system, moderation (kick/ban/timeout)
- Search (Tantivy + SearchModal), webhooks, slash commands, automod
- Events with RSVP, server discovery, forum channels, stage channels
- Role management UI, channel settings, server settings, audit log
- Emoji picker, custom server emojis, markdown rendering, message grouping
- Light/dark theme, cozy/compact display, admin dashboard, data export
- Push notifications (VAPID/FCM), channel permission overrides UI
- SMTP email integration (lettre) for password reset + verification
- **Security hardening (Sprint 0):**
  - Permission enforcement on all routes (compute_effective_permissions wired)
  - Per-endpoint rate limiting (axum-governor, per-IP token buckets)
  - JWT refresh token rotation + theft detection (JTI in DB, all sessions revoked on replay)
  - File upload security: MIME allowlist, size limits, magic-byte check, EXIF stripping
  - HTTP security headers (CSP, X-Frame-Options, X-Content-Type-Options, etc.)
  - 2FA / TOTP (RFC 6238, totp-rs; enable/verify/disable endpoints + client UI)
  - Audit log completeness: kick, role assign/remove added

Sprint 1 complete: ChannelCreate/Update/Delete, RoleCreate/Update/Delete, MemberUpdate, ServerUpdate all broadcast over WS and handled by client stores in real-time.
Sprint 2 complete: Message edit (inline textarea, Enter/Esc), delete, and "edited" indicator; edit/delete only shown for own messages.
Sprint 3 complete: Slowmode enforcement — `slowmode_delay INT` column in channels (migration 043), API enforces per-user cooldown on send_message (returns 429 with retry_after), ChannelSettingsModal exposes slowmode input (0–21600 seconds).
Sprint 4 complete: User profile popover — click any username/avatar in chat → inline card shows avatar, status dot, bio, role chips (color-coded), "Send Message" DM button.
Sprint 5 complete: UX polish — video/audio inline playback, category collapsing, server unread badges, status picker, emoji shortcodes (:smile: → 😄), quick switcher (Ctrl+K extends to channels+users).
Sprint 6 complete: Voice/video quality — device selection (mic/camera/speaker via enumerateDevices), video grid (CSS grid per participant count), per-participant volume slider.
Sprint 7 complete: Keyboard shortcuts — Alt+↑/↓ channel navigation, ↑ edit last message, Ctrl+Shift+M mute, Ctrl+K quick switcher, all wired via central keydown listener in +layout.svelte.

**SMTP email live (2026-03-28):** Password reset and verification emails now send via send.one.com (SMTPS port 465). Systemd service (`opencorde-api.service`) installed for auto-start on boot.

**Tauri desktop packaging (Phase 4) complete (2026-03-28):** .deb/.rpm packages build successfully. GitHub Actions CI/CD workflow for multi-platform releases (Windows/macOS/Linux) in `.github/workflows/release.yml`.

**E2EE (Phase 4) complete (2026-03-28):** Full OpenMLS end-to-end encryption wired:
- `e2ee_enabled` flag on channels (migration 047), toggle in ChannelSettingsModal
- `crypto_init` + key package upload on every login (Tauri only)
- Text messages encrypted with `enc:<hex>` prefix; decrypted on fetch + WebSocket receive
- Files encrypted with AES-256-GCM, `.enc` extension marker
- Voice E2EE via LiveKit ExternalE2EEKeyProvider (MLS epoch key export)
- Auto-join E2EE group when entering a channel (fetches pending welcome)
- Lock icon (🔒) on E2EE channels in channel list

**All planned sprints complete. Phase 4 complete.**

## Tech Stack (Exact Versions)

| Layer          | Technology                    | Version / Notes                        |
|----------------|-------------------------------|----------------------------------------|
| **Backend**    | Rust                          | 1.75+                                  |
|                | Axum                          | 0.8                                    |
|                | Tokio                         | 1.x (async runtime)                    |
|                | tracing                       | 0.1 (structured logging)               |
|                | thiserror / anyhow            | Error handling                         |
| **Database**   | PostgreSQL                    | 16+                                    |
|                | sqlx                          | 0.8 (compile-time query checking)      |
|                | migrations                    | SQLx migrations (SQL files)            |
| **Cache/PubSub** | Redis                        | 7+                                     |
| **File Storage** | MinIO                        | (S3-compatible object storage)         |
| **Voice/Video**  | LiveKit                      | Self-hosted SFU                        |
| **Desktop**    | Tauri                         | 2.0                                    |
|                | SvelteKit                     | 2.x                                    |
|                | Tailwind CSS                  | 4.x                                    |
|                | LiveKit JS SDK                | (client-side WebRTC)                   |
| **E2EE (Phase 2)** | OpenMLS                    | RFC 9420, Rust implementation          |
|                | Tantivy                       | Embedded full-text search              |

## How to Run

### Development (local)

```bash
# Start all services (PostgreSQL, Redis, MinIO, LiveKit)
docker compose up -d

# Rust backend tests
cargo test --workspace

# Watch and rebuild
cargo watch -x check

# Frontend (after backend is running)
cd client && pnpm install && pnpm dev
```

### Production

See [`docs/deployment.md`](./docs/deployment.md) for the web-client/static-host routing rules and deployment checklist.

---

## Conventions

All developers MUST adhere to these to keep the codebase navigable for AI agents:

### Rust (`server/`)
- **File limit:** 300 lines max per file. Break large modules into submodules.
- **Struct organization:** One logical struct per file (preferred). Impl blocks in same file.
- **File naming:** `snake_case.rs` (e.g., `user_repository.rs`, `permission_bitfield.rs`)
- **Error handling:** Use `thiserror` for domain errors; `anyhow` for early prototyping. All public functions must return `Result<T, Error>`.
- **Async runtime:** Tokio throughout. Mark async functions with `#[tokio::main]` or use within Axum handlers.
- **Logging:** Use `tracing` crate. Structured fields preferred: `tracing::info!(user_id = %id, "action description")`.
- **Testing:** Unit tests in same file in `#[cfg(test)]` modules. Integration tests in `tests/` directory.
- **Module structure:**
  ```
  src/
    lib.rs (exports public API)
    models/ (INDEX.md required)
      user.rs
      server.rs
      ...
    repositories/ (INDEX.md required)
      user_repository.rs
      ...
    handlers/ (INDEX.md required, Axum handlers only)
      auth.rs
      servers.rs
      ...
    middleware/ (INDEX.md required)
    utils/ (INDEX.md required)
    error.rs (custom error type)
  ```

### TypeScript/SvelteKit (`client/src/`)
- **File naming:** `PascalCase.svelte` for components; `camelCase.ts` for utilities/stores.
- **Route naming:** `kebab-case` directories (e.g., `+page.svelte` in `src/routes/auth/login/`).
- **Stores:** `export const store = writable()` in `src/lib/stores/`. File names match store purpose (e.g., `authStore.ts`).
- **File limit:** 300 lines per file.
- **Linting:** `pnpm lint` must pass. Use ESLint + Prettier config from root.

### Documentation
- **Markdown headers:** Use `# H1`, `## H2`, `### H3`. Never skip levels.
- **Code blocks:** Always include language identifier: ` ```rust ` or ` ```toml `.
- **Links:** Absolute paths from root (e.g., `[decisions](./decisions.md)`).

### Commit messages
- Atomic: one feature/fix per commit.
- Format: `type(scope): description` (e.g., `feat(auth): add JWT validation middleware`)
- Reference tasks: `Closes #task-id` in body.

---

## Key Decision Log

See [`decisions.md`](./decisions.md) for rationales behind:
- PostgreSQL (not MongoDB/SQLite)
- Rust + Axum (not Node.js)
- LiveKit (not Janus/Mediasoup)
- Tauri 2.0 (not Electron)
- SvelteKit (not React/Vue)
- Snowflake IDs
- AGPL-3.0 license
- Monorepo with Cargo workspace
- OpenMLS for E2EE
- Redis for pub/sub + cache

---

## Quick Links

- **Project map (YAML):** [`project-map.yaml`](./project-map.yaml)
- **Task tracker:** [`tasks.md`](./tasks.md)
- **Quality gate:** [`quality-gate.md`](./quality-gate.md)
- **Architecture decisions:** [`decisions.md`](./decisions.md)
- **Environment variables:** [`.env.example`](./.env.example)
- **Docker services:** [`docker-compose.yml`](./docker-compose.yml)

---

**Status:** Phases 1–4 complete. 47 migrations applied. E2EE operational. Desktop packages building. Last updated: 2026-03-28.

Last updated: 2026-03-22
