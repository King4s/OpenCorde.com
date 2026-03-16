# /client/src/lib/

Purpose: Shared SvelteKit library code for the frontend.

Pattern: Organized by concern — API communication, state management, real-time (LiveKit), crypto (Phase 2).

| Directory | Purpose |
|-----------|---------|
| api/ | API client modules (HTTP + WebSocket) |
| stores/ | Svelte stores for client state (auth, servers, messages, voice) |
| livekit/ | LiveKit SDK integration for voice/video |
| crypto/ | Client-side E2EE (Phase 2 placeholder) |
| components/ | Reusable Svelte UI components |
