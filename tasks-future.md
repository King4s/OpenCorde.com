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
- [ ] Recording (LiveKit Egress)
- [x] Steam OAuth login (OpenID 2.0 flow, migration 036, ghost user creation)

---

**Note:** Federation via Matrix protocol is explicitly excluded per user decision.

**Last updated:** 2026-03-24
