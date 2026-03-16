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

**Status:** COMPLETE

### Message Endpoints
- [x] Message routes (routes/messages/) — send, list (cursor pagination), edit, delete
- [x] Typing indicator (POST /api/v1/channels/{id}/typing)
- [x] Message repo already done in Weeks 1-2
- [ ] File attachment API (deferred — needs MinIO integration)

### WebSocket Gateway
- [x] Connection handler (ws/handler.rs) — HELLO → IDENTIFY → READY lifecycle
- [x] JWT auth on IDENTIFY, 10s timeout
- [x] 30-second heartbeat with tokio::select!
- [x] Event serialization helpers (ws/events.rs) — 10 event types
- [ ] Redis pub/sub dispatch (deferred — needs Redis connection)

### Quality Gate
- [x] 207 tests, 0 warnings, all files under 300 lines

**Last updated:** 2026-03-16
