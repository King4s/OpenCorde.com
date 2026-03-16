# Tasks: Weeks 9-12 (Voice, Desktop Client)

## Weeks 9-10: Voice & Video

**Objective:** LiveKit integration, voice state management, screen sharing.

### LiveKit Token Endpoint

- [ ] Voice handler (opencorde-api/src/routes/voice.rs)
  - POST /api/v1/livekit/token
  - Validate user is in voice channel, sign LiveKit token

### Voice State Management

- [ ] Voice state repository (opencorde-db/src/repos/voice_state_repo.rs)
  - join_voice, leave_voice, update_voice_state, get_channel_participants
- [ ] WebSocket voice events
  - VOICE_STATE_UPDATE dispatch
  - Auto-remove on disconnect

### Screen Sharing

- [ ] LiveKit handles natively — validate room membership only

### Testing

- [ ] Integration tests for voice endpoints
- [ ] E2E test: connect to LiveKit, publish/receive streams
- [ ] Run quality gate checks

---

## Weeks 11-12: Desktop Client (SvelteKit + Tauri)

**Objective:** Complete Tauri desktop app with UI.

### Project Setup

- [ ] Initialize Tauri 2.0 + SvelteKit project
  - Configure Vite for Tauri dev/prod builds

### Authentication UI

- [ ] Login page (/login) — email + password form
- [ ] Register page (/register) — username + email + password
- [ ] Auth store (authStore.ts) — tokens, auto-refresh, login/logout

### Server & Channel UI

- [ ] Server list sidebar
- [ ] Channel list panel
- [ ] Server store (serverStore.ts)
- [ ] Channel store (channelStore.ts)

### Messaging UI

- [ ] MessageList.svelte — infinite scroll, file attachments
- [ ] MessageInput.svelte — text, file upload, typing indicator
- [ ] Message store (messageStore.ts) — pagination, WebSocket events
- [ ] WebSocket client (websocket.ts) — IDENTIFY, READY, heartbeat

### Voice/Video UI

- [ ] Voice channel panel with video grid
- [ ] VideoGrid.svelte — LiveKit participants
- [ ] LiveKit client (room.ts) — connect, publish, receive
- [ ] Voice store (voiceStore.ts) — room state, mute/deafen

### App Settings & Tray

- [ ] Settings page — profile, theme, notifications
- [ ] System tray — show/hide, minimize to tray, unread badge
- [ ] App lifecycle — save/restore window state

### Styling

- [ ] Tailwind CSS 4.x — responsive, dark mode
- [ ] Reusable components — Button, Input, Modal, Avatar

### Testing

- [ ] Manual testing: full user flow
- [ ] Build for Windows, macOS, Linux
- [ ] Binary size check (<100MB)
- [ ] Run quality gate checks

**Last updated:** 2026-03-16
