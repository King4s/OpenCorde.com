# /crates/opencorde-api/src/ws/

Purpose: WebSocket gateway for real-time events.

Pattern: Separated by concern — connection handling, event definitions, message fan-out.

| File | Purpose |
|------|---------|
| handler.rs | WebSocket upgrade handler, connection lifecycle |
| events.rs | Event type definitions (e.g., MessagePosted, UserOnline, VoiceStateChanged) |
| dispatch.rs | Fan-out logic — routes events to connected clients |
| mod.rs | Module exports |
