# /crates/opencorde-api/src/routes/

Purpose: HTTP route handlers for the REST API.

Pattern: One file per resource. Each file exports a `router()` function that returns an Axum Router with AppState.

## Current (Phase 1-2)

| File | Purpose | Routes | Status |
|------|---------|--------|--------|
| health.rs | Health check endpoint | GET /api/v1/health | ✓ Implemented |
| auth.rs | Authentication endpoints | POST /api/v1/auth/register, /api/v1/auth/login, /api/v1/auth/refresh | ✓ Implemented |
| users.rs | User profile endpoints | GET /api/v1/users/@me, PATCH /api/v1/users/@me | ✓ Implemented |
| servers.rs | Server management | GET/POST /api/v1/servers, GET/PATCH/DELETE /api/v1/servers/{id} | ✓ Implemented |
| channels.rs | Channel management | GET/POST /api/v1/servers/{server_id}/channels, PATCH/DELETE /api/v1/channels/{id} | ✓ Implemented |
| mod.rs | Router composition, exports | Combines all routers | ✓ Implemented |

## Planned (Phase 2-3)

| File | Purpose | Routes | Status |
|------|---------|--------|--------|
| messages.rs | Message endpoints | GET/POST /channels/{channelId}/messages, GET/PUT/DELETE /messages/{id} | — |
| voice.rs | Voice session management | POST /voice/sessions, DELETE /voice/sessions/{id} | — |
| files.rs | File upload/download | POST /files/upload, GET /files/{id} | — |
| members.rs | Server member management | GET/POST /servers/{server_id}/members, DELETE /servers/{server_id}/members/{user_id} | — |
| invites.rs | Server invites | GET/POST /servers/{server_id}/invites, POST /invites/{code}/join | — |
| admin.rs | Admin operations | System config, user management, audit logs | — |
| push.rs | Push notification token registration | POST /api/v1/push/register, DELETE /api/v1/push/unregister | ✓ Implemented |

## Notes
- Every handler returns `Result<T, ApiError>` for unified error handling
- Handlers use tracing::instrument for structured logging
- Handlers extract State<AppState> for database and config access
- All routes use /api/v1 prefix for versioning
