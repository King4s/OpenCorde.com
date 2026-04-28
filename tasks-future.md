# OpenCorde — Phase 4+ Tasks

> Legacy implementation log, not Discord parity proof.
>
> Checked items in this file mean the historical task was implemented or claimed at the time.
> They do not mean the feature is complete, polished, permission-safe, realtime-safe, mobile-safe,
> or Discord-parity proven. Use `docs/plans/2026-04-28-discord-parity-master-plan.md` and
> `reports/discord-parity.json` as the current source of truth.

Phases 1, 2, and 3 had a passing historical smoke baseline as of 2026-03-22
(26/26 browser tests, 30 migrations). That is not a current Discord-parity claim.

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
- [x] Announcement channel type (type 4) — megaphone icon in ChannelList, accepted by API validation
- [x] Discord server structure import — Danish-Truckers.com (8 categories, 57 channels, 24 roles) imported via scripts/import_discord_server.py

### Gap Analysis: Discord Features Not Yet in OpenCorde
- [x] Server onboarding (GUILD_ONBOARDING) — GET/PUT /onboarding endpoint, OnboardingModal.svelte (shown once per session), OnboardingPanel in settings
- [x] Server guide (GUILD_SERVER_GUIDE) — server home page now shows channel overview, member count, welcome message
- [x] Soundboard — GET/POST/DELETE/play endpoints (migration 048), SoundboardPanel.svelte in voice sidebar
- [x] Verification level enforcement (field exists; enforced on send_message and join via invite)

---

## Phase 6: Security Hardening & UX Polish (legacy status from 2026-04-01)

### Security
- [x] Permission enforcement — `require_channel_perm` / `require_server_perm` wired to all relevant routes
- [x] Per-endpoint rate limiting — `axum-governor` (login 5/min, register 3/min, messages 5/sec, files 10/min, etc.)
- [x] JWT refresh token rotation — JTI tracking + theft detection (revoke all tokens on replay)
- [x] 2FA / TOTP — totp-rs (RFC 6238), enable/verify/disable endpoints, login integration
- [x] File upload security — MIME type allowlist, magic byte verification, EXIF strip, 8MB/100MB/25MB limits
- [x] XSS prevention — MarkdownContent.svelte: script/iframe/event-handler/javascript:-URL stripping
- [x] HTTP security headers — CSP, X-Frame-Options, X-Content-Type-Options, Referrer-Policy, Permissions-Policy
- [x] SQL injection audit — all queries use sqlx parameterized binds, no raw interpolation
- [x] Audit log completeness — channel, role, webhook, server, 2FA events logged (2fa.enable, 2fa.disable, server.update, server.delete, channel.create/update/delete, role.create/update/delete, webhook.create/delete)

### Voice / Video
- [x] Video grid (multi-participant) — VideoGrid.svelte: responsive columns, speaking ring, volume slider per participant
- [x] Voice device selection — VoiceSettings.svelte: mic/camera/speaker via enumerateDevices(), persisted to localStorage

### UX
- [x] Quick switcher extended — SearchModal now searches channels + members in addition to messages
- [x] Keyboard shortcuts — Alt+↑/↓ channel navigation, Ctrl+K quick switcher (wired in server layout)
- [x] WebSocket events — ChannelCreate/Update/Delete, RoleCreate/Update/Delete, MemberUpdate, ServerUpdate broadcast to all subscribers

---

**Note:** Federation via Matrix protocol is explicitly excluded per user decision.

**Last updated:** 2026-04-01 legacy log. Current parity status is tracked in `reports/discord-parity.json`.
