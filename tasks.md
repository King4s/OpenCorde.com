# OpenCorde Task Tracker

> Legacy implementation log, not Discord parity proof.
>
> Checked items in this file mean the historical task was implemented or claimed at the time.
> They do not mean the feature is complete, polished, permission-safe, realtime-safe, mobile-safe,
> or Discord-parity proven. Use `docs/plans/2026-04-28-discord-parity-master-plan.md` and
> `reports/discord-parity.json` as the current source of truth.

Organized by phase and week. Check items off as they complete.

**Split files** (to stay under 300 lines per file):
- **This file**: Week 0 (done) + Weeks 1-2 (done)
- [tasks-weeks-3-8.md](./tasks-weeks-3-8.md): Auth, Servers, Messaging — COMPLETE
- [tasks-weeks-9-12.md](./tasks-weeks-9-12.md): Voice, Desktop Client — COMPLETE
- [tasks-future.md](./tasks-future.md): Phase 4+ (Tauri, E2EE, Bridge)

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

#### Infrastructure

- [x] Verify docker-compose services start on dev server
- [x] Install Rust toolchain on dev server (192.168.140.140)
- [x] Install pnpm on dev server

---

## Phase 1-3 Summary (Legacy Implementation Status as of 2026-03-22)

At the time, 26/26 browser smoke tests passed and 30 migrations were applied. This does not prove
Discord parity. Treat historical feature wording below as context only until each workflow
has current Playwright/API/permission/mobile/Emma evidence.

### Sprint 2026-03-22: SMTP + Docs
- [x] Fix validation test bug (validate_channel_type type 3 = Stage is valid; test type 4)
- [x] Implement lettre SMTP in email.rs (replaces logging stub)
- [x] Add SMTP vars to .env.example
- [x] Update ReadMeFirst.md, tasks.md, tasks-future.md

### Sprint 0: Full Security Hardening (2026-03-24 to 2026-03-25)
- [x] 0a: Permission enforcement — wired compute_effective_permissions() into all routes (messages, channels, reactions, threads, members, invites, roles, webhooks)
- [x] 0b: Per-endpoint rate limiting — axum-governor with per-IP token buckets; 5/min login, 3/min register, 5/sec messages, 10/min uploads, 60/min global
- [x] 0c: JWT refresh token rotation — JTI (UUID v4) stored in refresh_tokens table; single-use; stolen token detection revokes all sessions; migration 041
- [x] 0d: File upload security — MIME allowlist, per-category size limits (image 8MB, video 100MB, other 25MB), magic-byte verification, EXIF stripping (img-parts, lossless)
- [x] 0e: XSS audit — custom markdown renderer already safe (escapeHtml() + code-block extraction); no DOMPurify needed
- [x] 0f: HTTP security headers middleware — X-Content-Type-Options, X-Frame-Options, Referrer-Policy, Permissions-Policy, Content-Security-Policy
- [x] 0g: SQL injection audit — zero format!() in repos; all queries use .bind() parameterization
- [x] 0h: 2FA (TOTP) — migration 042, POST /auth/2fa/enable + verify + DELETE /auth/2fa, login requires totp_code when enabled; client: TwoFactorSetup.svelte, login TOTP step, settings section
- [x] 0i: Password security audit — Argon2id confirmed, 8-char minimum enforced, delete account requires current password
- [x] 0j: Audit log completeness — added member.kick, member.role_assign, member.role_remove; existing: ban, unban, timeout, timeout_removed

### Sprint 1: Missing WebSocket Events (2026-03-25)
- [x] 1a: ws/dispatch.rs — add `member_server_ids: HashSet<i64>` param; route ChannelCreate/Update/Delete, RoleCreate/Update/Delete, MemberUpdate, ServerUpdate to server-member filter
- [x] 1b: ws/handler/main_loop.rs — thread `member_server_ids` through to `should_dispatch`
- [x] 1c: ws/handler/lifecycle.rs — load user's server IDs via `server_repo::list_by_user`; pass to `run_main_loop`
- [x] 1d: ws/events.rs — add 7 new event builders (channel_update, channel_delete, role_create, role_update, role_delete, member_update, server_update)
- [x] 1e: channels/handlers.rs — broadcast ChannelCreate/Update/Delete after successful CRUD
- [x] 1f: servers/handlers/crud.rs — broadcast ServerUpdate after successful PATCH
- [x] 1g: roles.rs — broadcast RoleCreate/Update/Delete + MemberUpdate (assign/unassign)
- [x] 1h: client stores — initChannelListeners, initRoleListeners, initMemberListeners, initServerListeners; called from +layout.svelte

### Sprint 2: Message Edit/Delete UI (2026-03-25)
- [x] 2a: messages store — added editMessage(messageId, content) and deleteMessage(messageId) API calls
- [x] 2b: MessageContextMenu.svelte (new) — extracted context menu buttons + added Edit (✏) and Delete (🗑) for own messages only
- [x] 2c: MessageList.svelte — added onEdit/onDelete/currentUserId props, inline edit mode (textarea, Enter saves, Esc cancels), uses MessageContextMenu component
- [x] 2d: channel +page.svelte — wired handleEditMessage, handleDeleteMessage, passes currentUserId to MessageList

---

## Tracking Notes

- Each task must pass quality-gate.md before completion
- Update ReadMeFirst.md with progress
- Blockers noted inline with `BLOCKED: reason`

### Sprint 3: Slowmode Enforcement (2026-03-25)
- [x] 3a: migration 043_slowmode.sql — ALTER TABLE channels ADD COLUMN slowmode_delay INT NOT NULL DEFAULT 0
- [x] 3b: channel_repo.rs — add slowmode_delay to ChannelRow; update_channel accepts slowmode_delay param
- [x] 3c: channels/types.rs — add slowmode_delay to ChannelResponse and UpdateChannelRequest
- [x] 3d: channels/handlers.rs — map slowmode_delay in channel_row_to_response; clamp 0–21600; pass to update_channel
- [x] 3e: send_list.rs — after channel fetch, if slowmode_delay > 0 query user's last message; return RateLimited{retry_after} if too soon
- [x] 3f: types.ts — add slowmode_delay to Channel interface
- [x] 3g: ChannelSettingsModal.svelte — add slowmodeDelay state, number input (0–21600), include in PATCH body
- [x] 3h: channel +page.svelte — pass channelSlowmode to modal, update store on save

---

### Sprint 4: User Profile Popup (2026-03-25)
- [x] 4a: role_repo.rs — add list_by_member(user_id, server_id) → Vec<RoleRow> via JOIN on member_roles
- [x] 4b: roles.rs — GET /servers/{server_id}/members/{user_id}/roles returns Vec<RoleResponse>
- [x] 4c: UserProfilePopover.svelte (new) — avatar, status dot, bio, role chips, Send Message DM button
- [x] 4d: MessageList.svelte — avatar div and author name wrapped in clickable buttons, open popover on click
- [x] 4e: channel +page.svelte — pass serverId prop to MessageList

---

**Last updated:** 2026-03-25
