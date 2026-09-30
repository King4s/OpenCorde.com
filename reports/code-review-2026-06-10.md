# Code Review & Discord Feature-Gap Report — 2026-06-10

Full-codebase review of OpenCorde (backend crates + SvelteKit client) plus a feature-gap
analysis against Discord. Findings only — no code was changed. Implementation priorities
are at the end; the deep platform work (bots/OAuth2/interactions) is already planned in
`.hermes/plans/2026-06-09_183653-opencorde-discord-platform-parity.md` and is referenced,
not duplicated, here.

---

## 1. Critical findings (security / correctness)

### 1.1 OAuth2 token endpoint returns a mock token

`crates/opencorde-api/src/routes/oauth2/token.rs:27-29`:

```rust
// TODO: Full implementation — validate client, code, issue JWT
Json(TokenResponse {
    access_token: "mock_token_placeholder".into(),
```

The endpoint performs no client validation, no authorization-code check, and issues a
hardcoded fake token. Any client or test trusting this endpoint is silently broken, and
the route must not be reachable in production until implemented. Corresponds to
parity-plan Task 9 (OAuth2 authorize/token endpoints).

### 1.2 Invite-only registration does not validate invite codes

`crates/opencorde-api/src/register.rs:47`:

```rust
// TODO: validate invite code once invite system supports this mode
```

Registration in invite-only mode accepts any invite code. The invite system itself
(expiry/max-uses) exists, so wiring the validation is unblocked.

### 1.3 2FA has no recovery codes

TOTP enable/verify/disable is implemented and tested, but there is no backup/recovery-code
mechanism. A user who loses their device is permanently locked out unless an admin
intervenes directly in the database.

---

## 2. Convention violations — 300-line rule (17 files)

The repo convention is hard: no source file over 300 lines, split at ~250.

**Backend (9 files):**

| File | Lines |
|---|---:|
| `crates/opencorde-api/src/routes/messages/send_list.rs` | 680 |
| `crates/opencorde-api/src/routes/roles.rs` | 605 |
| `crates/opencorde-api/src/routes/admin/upgrade.rs` | 602 |
| `crates/opencorde-api/src/routes/admin/jobs.rs` | 596 |
| `crates/opencorde-api/src/routes/admin/branding.rs` | 557 |
| `crates/opencorde-api/src/routes/dms/handlers.rs` | 546 |
| `crates/opencorde-api/src/routes/admin/backup.rs` | 536 |
| `crates/opencorde-api/src/routes/admin/storage.rs` | 445 |
| `crates/opencorde-api/src/routes/voice/handlers.rs` | 421 |

Natural splits: `send_list.rs` → `send_message.rs` + `list_messages.rs`; `roles.rs` →
`roles/` module with one handler group per file; `admin/upgrade.rs` → extract the
~116-line version-resolution logic into its own module.

**Frontend (8 files):**

| File | Lines |
|---|---:|
| `client/src/lib/components/user/StatusPicker.svelte` | 535 |
| `client/src/lib/components/chat/MessageList.svelte` | 479 |
| `client/src/routes/admin/BrandingPanel.svelte` | 411 |
| `client/src/routes/admin/FederationPanel.svelte` | 387 |
| `client/src/routes/admin/JobsPanel.svelte` | 378 |
| `client/src/lib/components/chat/MessageInput.svelte` | 335 |
| `client/src/routes/admin/UpgradePanel.svelte` | 319 |
| `client/src/lib/components/chat/WaveformPlayer.svelte` | 309 |

`StatusPicker` bundles status selection, custom status, activity type and expiry —
splits cleanly into 3-4 sub-components.

---

## 3. Backend quality

**Issues:**

- Non-test `.unwrap()`/`.expect()` calls worth converting to proper error propagation in
  `routes/unfurl.rs` (7) and `routes/voice/livekit.rs` (5). The remaining ~120 across
  opencorde-api are in `#[cfg(test)]` code or guaranteed-valid static values
  (e.g. `security_headers.rs`).
- No workspace clippy lint configuration exists (no `[workspace.lints]`, no
  `.cargo/config.toml`), even though `quality-gate.md` requires
  `cargo clippy --workspace -- -D warnings`. 11 scattered `#[allow(...)]` suppressions,
  including 6 `#[allow(dead_code)]` on type fields — worth a pass to delete or justify.
- Thin test coverage on new code: `repos/dm_repo.rs` is 468 lines with 3 tests; the new
  platform repos (bot tokens, interactions, embeds, integration logs) are largely
  untested despite the parity plan's TDD requirement.

**Positives (keep doing this):**

- Argon2id hashing for passwords, bot tokens, OAuth client secrets and webhook tokens —
  no plaintext credential storage or comparison found.
- Parameterized SQL throughout; no string-built queries.
- Permission checks centralized in `permission_check::require_server_perm` /
  `require_channel_perm` and used consistently across routes — no copy-paste drift.
- WebSocket gateway is cleanly modularized (lifecycle / main_loop / dispatch / events,
  all under 320 lines).

---

## 4. Frontend quality

- **Silent error swallowing** (~10+ empty/noop catch blocks) despite an existing toast
  system (`stores/toasts.svelte.ts`, `toastError()`):
  - `stores/auth.ts` — 4× bare `catch {}` around token storage/restore/refresh
  - `stores/voice.ts` — 9× noop catches; voice-session network failures are invisible
  - `stores/messages.ts` — decryption failure renders `"[Decryption failed]"` with no
    user notification
  - `UserProfilePopover.svelte:78` — clipboard copy failure swallowed
- **No message-list virtualization**: `MessageList.svelte` renders every loaded message
  in the DOM and has no lazy-load of older history while scrolling up. Jank risk on long
  channels and low-end devices; Discord virtualizes.
- **Half-finished Svelte 5 migration**: 25 of 27 stores use legacy
  `writable()/derived()` while newer components use `$state` runes. Pick a direction and
  finish it — the mix makes reactivity behavior inconsistent.
- **5 stub settings panels** rendering "coming soon": `AuthorizedAppsPanel`,
  `SessionsPanel` (no "log out other devices"), `ActivityStatusPicker`,
  `AccountSwitcher`, `UserBadges`.
- **Token storage fallback**: on Tauri the keychain is used, but the fallback path stores
  the access token in plaintext localStorage (`stores/auth.ts`).
- No hardcoded secrets or internal URLs found in client code.

---

## 5. Discord feature-gap matrix

### 5.1 Already planned — defer to the platform-parity plan

Partially built in the last 21 commits (mostly DB + API, no UI yet): developer portal,
bot token management UI, bot gateway WS protocol with intents, application command
UI/autocomplete, interaction lifecycle (ephemeral/deferred/followup proof), embed
renderer, Discord-style `{webhook_id}/{token}` webhooks, integration-log API routes + UI
panel, automod beyond keyword-contains, app directory.

### 5.2 Backend exists, no API and/or UI

| Feature | Has | Missing |
|---|---|---|
| Polls | migration 069 | API routes + UI |
| Rich presence/activity | migration 065 | API + display in member list/profile |
| Message components (buttons/selects) | migration 077 table | API + renderer |
| Scheduled messages | migration 072 + routes | UI (compose, list, cancel) |
| DM message-request inbox | migration 070 + backend | UI tab (accept/ignore/spam) |
| Integration logs | migration 078 + repo | list API + settings panel |
| Recordings | routes + migration 037 | management UI |
| Data export | route | UI integration |

### 5.3 Missing entirely vs Discord

- **Group DMs** — no schema, no API. Biggest social gap.
- **GIF picker** (Tenor/Giphy-style) and **stickers**
- **Image lightbox** (full-screen attachment viewer) and **paste-image upload**
- **Jump-to-date** in message history; search filters (`from:`, `has:`, date range)
- **Granular notification settings UI** — per-channel/per-server mute, sounds, desktop
  alert levels (backend tables 066 exist; current UI is a single push toggle)
- **Role icons**, **server templates**, **vanity URLs**
- **User notes**, **member screening/rules acceptance**
- **2FA recovery codes**; **change-password-while-logged-in** endpoint
- **Server boosts/Nitro** — intentionally skipped (monetization, irrelevant self-hosted)

### 5.4 Strong / proven — leave alone

Messaging (send/edit/reply/react/pin/attach, two-client realtime proof), roles &
permissions (52/52 smoke tests), voice/video/stage (two-client WebRTC proof, adaptive
Opus), friends/DMs lifecycle, auth (19/19 QA checks, JTI theft detection, TOTP).

---

## 6. Recommended priority order

1. **OAuth2 token endpoint + invite-code validation** — security/correctness, small scope
2. **Surface swallowed frontend errors** through the existing toast system
3. **2FA recovery codes + logged-in password change** — account-safety basics
4. **File splits to restore the 300-line rule + workspace clippy lints** — pays down
   convention debt before the platform sprint adds more code
5. **Quick UX wins**: image lightbox, paste-to-upload, polls API+UI (backend exists)
6. **Continue the platform-parity plan** (UI layers for bots/commands/embeds/portal)
7. **Bigger features**: group DMs, presence display, granular notifications, virtualized
   MessageList
