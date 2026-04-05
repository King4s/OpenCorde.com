# /client/src/lib/api/

Purpose: API client modules for communicating with OpenCorde backend.

Pattern: Separated by transport — HTTP for REST, WebSocket for real-time events.

| File         | Purpose                                                      |
| ------------ | ------------------------------------------------------------ |
| client.ts    | HTTP client initialization, auth token management            |
| websocket.ts | WebSocket connection, event subscription, reconnection logic |
| types.ts     | TypeScript type definitions for API responses                |
