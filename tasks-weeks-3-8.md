# Tasks: Weeks 3-8 (Auth, Servers, Messaging)

## Weeks 3-4: Authentication

**Status:** COMPLETE

### User Registration & Login

- [x] Auth routes (opencorde-api/src/routes/auth/) — register, login, refresh
  - POST /api/v1/auth/register — validation, Argon2 hash, Snowflake ID, JWT
  - POST /api/v1/auth/login — email lookup, password verify, JWT
  - POST /api/v1/auth/refresh — HttpOnly cookie, token rotation
- [x] Argon2id password hashing (opencorde-core/src/password.rs) — 7 tests
- [x] JWT token management (opencorde-api/src/jwt.rs) — access + refresh, 5 tests
- [x] Auth middleware extractor (opencorde-api/src/middleware/auth.rs) — AuthUser from Bearer token

### User Profile

- [x] GET /api/v1/users/@me — authenticated profile retrieval
- [x] PATCH /api/v1/users/@me — username/email update with conflict detection
- [ ] POST /api/v1/users/@me/avatar — upload to MinIO (deferred to file upload task)

### Rate Limiting

- [ ] Token bucket rate limiter (deferred — requires Redis integration)

### Quality Gate

- [x] 141 tests passing (83 core + 8 db + 41 api + 9 integration)
- [x] 0 clippy warnings, formatted, all files under 300 lines

---

## Weeks 5-6: Servers & Channels

**Status:** COMPLETE

### Server Management
- [x] Server routes (routes/servers/) — POST, GET, GET/:id, PATCH, DELETE
- [x] Auto-add owner as member on create, ownership checks on mutate

### Channel Management
- [x] Channel routes (routes/channels/) — POST, GET, PATCH, DELETE
- [x] Channel type validation (Text/Voice/Category), parent category support

### Roles & Permissions
- [x] Role routes (routes/roles.rs) — CRUD + assign/unassign to members
- [x] Role repository (repos/role_repo.rs)
- [x] Ownership checks via helpers module
- [ ] Redis-cached permission computation (deferred)

### Invites
- [x] Invite routes (routes/invites.rs) — create, lookup (public), join, revoke
- [x] Invite repository (repos/invite_repo.rs)

### Server Membership
- [x] Member routes (routes/members.rs) — list, leave/kick, nickname

### Quality Gate
- [x] 168 tests, 0 warnings, all files under 300 lines

---

## Weeks 7-8: Messaging & WebSocket Gateway

**Objective:** Message CRUD, file attachments, WebSocket gateway, pub/sub.

### Message Endpoints

- [ ] Message handler (opencorde-api/src/routes/messages.rs)
  - POST /api/v1/channels/:id/messages (send, requires Send permission)
  - GET /api/v1/channels/:id/messages (cursor-based: before, after, limit)
  - PATCH /api/v1/messages/:id (edit, owner only)
  - DELETE /api/v1/messages/:id (owner or admin)
- [ ] Message repository with cursor-based pagination
- [ ] File attachment API
  - POST /api/v1/files/upload (multipart, store in MinIO)

### WebSocket Gateway

- [ ] Connection handler (opencorde-api/src/ws/handler.rs)
  - GET /api/v1/gateway (WebSocket upgrade)
  - IDENTIFY -> READY flow, heartbeat, session cleanup
- [ ] Event types (opencorde-api/src/ws/events.rs)
  - MESSAGE_CREATE/UPDATE/DELETE, TYPING_START
  - PRESENCE_UPDATE, VOICE_STATE_UPDATE
- [ ] Redis pub/sub dispatch (opencorde-api/src/ws/dispatch.rs)
  - Publish to server:{server_id} channels
  - Fan-out to connected WebSocket clients
- [ ] Typing indicator
  - POST /api/v1/channels/:id/typing
  - 5-second expiry, deduplication

### Testing

- [ ] Integration tests for messages
- [ ] WebSocket integration tests (connect, heartbeat, broadcast)
- [ ] Run quality gate checks

**Last updated:** 2026-03-16
