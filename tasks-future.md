# OpenCorde — Phase 4+ Tasks

Phases 1, 2, and 3 are complete as of 2026-03-22 (26/26 browser tests, 30 migrations).

## Phase 4: Differentiators

### Tauri Desktop Packaging
- [x] Wrap SvelteKit in Tauri 2.0 for distributable .exe/.dmg/.AppImage
- [x] System tray icon + OS-level desktop notifications
- [x] Auto-launch on startup
- [x] Deep links (opencorde:// protocol)

### End-to-End Encryption (OpenMLS)
- [x] Integrate OpenMLS library into opencorde-crypto (openmls 0.5, RFC 9420)
- [x] E2EE key management endpoints (key_packages + groups routes, migrations 031-032)
- [x] opencorde-crypto: key_package, group, encrypt modules (5/5 tests passing)
- [x] Client-side encryption/decryption (Tauri commands: crypto_init/create_group/add_member/process_welcome/encrypt/decrypt)
- [x] LiveKit E2EE for voice/video (ExternalE2EEKeyProvider + MLS epoch export key, auto key rotation)
- [x] AES-256-GCM file encryption (encrypt on upload, decrypt on click; key from MLS epoch exporter)

### Discord Bridge
- [x] opencorde-bridge: gateway connection to Discord (twilight-gateway 0.15, single shard)
- [x] Discord REST API client (twilight-http, webhook execute with username/avatar override)
- [x] Message/user/channel mapping (bridge_channel_mappings, migrations 034-035)
- [x] Ghost user management (bridge_ghost_users, auto-create on first message)
- [x] Bridge management API (GET/POST/PATCH/DELETE /api/v1/servers/{id}/bridge/mappings)
- [x] Bridge settings UI (IntegrationsPanel Discord Bridge section with live mapping management)
- [x] Bridge systemd service (opencorde-bridge.service, reads .env.bridge for DISCORD_TOKEN)

### SMTP: Account Verification
- [x] Extend email.rs: send verification email on register
- [x] Email verified flag on users table (migration 033)
- [x] Resend verification endpoint (POST /api/v1/auth/resend-verification)
- [x] Verify email endpoint (GET /api/v1/auth/verify-email?token=...)

---

## Phase 5: Mobile & Admin

- [ ] Mobile apps (Tauri 2.0 iOS/Android)
- [ ] Push notifications (mobile)
- [x] Instance admin: storage usage monitor, rate limiting config
- [x] GDPR data export improvements (unlimited messages, file attachments, account deletion)
- [x] Accessibility audit (WCAG 2.1 AA) — dialog tabindex, backdrop Escape, keyboard nav (AutomodManager, EmojiManager, SlashCommandManager, WebhookManager)
- [x] Recording (LiveKit Egress — start/stop/list endpoints + RecordingsPanel + VoicePanel button)
- [x] Steam OAuth login (OpenID 2.0 flow, migration 036, ghost user creation)

---

**Note:** Federation via Matrix protocol is explicitly excluded per user decision.
---

## Sprint Completions (2026-03-28)

### Security Hardening
- [x] Permission enforcement wired on all critical routes (require_channel_perm / require_server_perm)
- [x] Per-endpoint rate limiting (axum-governor — auth 5/min, messages 5/sec, files 10/min)
- [x] JWT refresh token rotation (refresh_token_repo, JTI tracking, theft detection)
- [x] File upload validation (MIME type, magic bytes, size limits per type, EXIF strip)
- [x] XSS prevention (DOMPurify in MarkdownContent, marked sanitizer)
- [x] HTTP security headers (SecurityHeaders middleware: CSP, X-Frame-Options, etc.)
- [x] 2FA TOTP (totp.rs routes, TwoFactorSetup/TwoFactorModal components, login gate)
- [x] Argon2id password hashing verified; minimum length enforced
- [x] Audit log completeness (role changes, permission overrides, ban/kick/timeout, webhooks)

### WebSocket Events
- [x] ChannelCreate / ChannelUpdate / ChannelDelete broadcast + client store handlers
- [x] RoleCreate / RoleUpdate / RoleDelete broadcast + client store handlers
- [x] MemberUpdate (role assign/remove) broadcast + client store handler
- [x] ServerUpdate broadcast + client store handler

### UX Completions
- [x] Message edit inline (textarea in MessageList, ↑ in empty input to edit last message)
- [x] Message delete (context menu → confirm → DELETE /messages/{id})
- [x] Slowmode enforcement (last_message_at check in send_message, 429 if too fast)
- [x] User profile popover (UserProfilePopover.svelte — avatar, roles, DM button)
- [x] Video/audio inline playback (<video>/<audio> based on MIME type)
- [x] Category collapsing (ChannelList groups by parent_id, click to toggle)
- [x] Server unread badges (red dot on ServerIcon when any channel has unread)
- [x] Status picker in UserPanel (Online/Idle/DND/Invisible + PATCH /users/@me)
- [x] Quick switcher Ctrl+K (QuickSwitcher.svelte — channels, servers, users)
- [x] Alt+↑/↓ channel navigation (fixed missing channels import in serverId layout)

### Voice/Video Quality
- [x] Voice device selection (VoiceSettings.svelte — mic/cam/speaker via enumerateDevices)
- [x] Video grid (VideoGrid.svelte — multi-participant with CSS grid, per-participant volume)
- [x] Per-participant volume slider (LiveKit participant.setVolume)

### Keyboard Shortcuts
- [x] Alt+↑/↓ navigate channels
- [x] ↑ in empty input — edit last own message
- [x] Ctrl+K quick switcher
- [x] Ctrl+, settings
- [x] Alt+Home DMs
- [x] Esc close modal

### Discord Mention Rendering (in MarkdownContent.svelte)
- [x] <#channelId> → channel name chip with lookup
- [x] <@userId> / <@!userId> → @username chip
- [x] <@&roleId> → role name chip with role color
- [x] @everyone / @here mentions
- [x] <t:timestamp:R> → relative time display

**Last updated:** 2026-03-28
