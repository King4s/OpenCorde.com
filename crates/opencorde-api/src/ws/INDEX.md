# /crates/opencorde-api/src/ws/

Purpose: WebSocket gateway for real-time events.

Pattern: Separated by concern — connection handling, event serialization.

## Architecture

Follows Discord's gateway model:
1. Client connects → server sends HELLO
2. Client sends IDENTIFY with JWT → server validates and sends READY
3. Periodic heartbeats keep connection alive
4. Server pushes events to all connected clients

## Files

| File | Purpose | Lines |
|------|---------|-------|
| handler.rs | WebSocket upgrade handler, connection lifecycle | ~210 |
| events.rs | Event serialization helpers (MESSAGE_CREATE, TYPING_START, etc.) | ~160 |
| mod.rs | Module exports | ~13 |

## Future Work

- `dispatch.rs` — Fan-out logic to route events to specific users/servers
- Connection registry — track connected clients per user/server
- Presence updates — broadcast user online/offline status
- Voice signaling — coordinate with LiveKit
