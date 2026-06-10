# OpenCorde 100% and Beyond — Fixes + Missing Features Implementeringsplan

> **Til Hermes/AI-agent:** Eksekvér med subagent-driven-development. Læs `ReadMeFirst.md` og `reports/code-review-2026-06-10.md` (kildegrundlag for denne plan) først. Arbejd task-for-task, TDD hvor muligt. Ingen produktionsændringer uden Marcins eksplicitte godkendelse.

**Mål:** Luk alle fund fra code-review 2026-06-10: sikkerhedsfejl, konventionsgæld, frontend-robusthed, sovende backend-features (DB findes, ingen API/UI) og manglende Discord-features — og derefter "beyond": ting Discord ikke kan (E2EE-bevis, federation-bevis).

**Afgrænsning mod eksisterende plan:** Platform-laget (developer portal, bot gateway/intents, application commands UI, interactions-bevis, embed renderer, Discord-webhooks, integration-log UI, automod-udvidelse, app directory) er ALLEREDE planlagt i `.hermes/plans/2026-06-09_183653-opencorde-discord-platform-parity.md` (Tasks 1–40). Denne plan duplikerer dem IKKE — den refererer dem. Hvor en task her overlapper, står det eksplicit.

**Nuværende tilstand:** Se `reports/code-review-2026-06-10.md`. Kernen (messaging, roller, voice/stage, friends/DMs, auth) er proven. Kritiske fund: mock OAuth2 token endpoint, manglende invite-validering, ingen 2FA recovery codes. 17 filer bryder 300-linjers reglen. Frontend sluger fejl. Polls/presence/components/scheduled/message-requests har backend uden UI. Group DMs, GIF/stickers, lightbox m.fl. mangler helt.

**Risici:**
- **Regression i proven områder:** Filsplits (Fase B) rører `send_list.rs`, `roles.rs`, `MessageList.svelte` — alle proven workflows. Splits skal være rene moves uden logikændringer, verificeret med eksisterende tests + Playwright smoke.
- **Svelte 5 store-migrering:** 25 stores på legacy `writable()`. Big-bang migrering er farlig; migrér én store ad gangen med komponent-følgesvende.
- **Token/secret håndtering:** Recovery codes, vanity codes og GIF-provider-nøgler er secrets/semi-secrets. Hashing/serverside-proxy som beskrevet pr. task. Ingen secrets i repo/rapporter.
- **Migration disciplin:** Aldrig redigér anvendte migrationer; kun nye numre (næste ledige: 079+).
- **300-linjers reglen gælder også ny kode** — split tidligt.

---

# Faser

- **Fase A** — Kritiske sikkerheds-/korrekthedsfixes (Tasks 1–5)
- **Fase B** — Konventions- og kvalitetsgæld, backend (Tasks 6–10)
- **Fase C** — Frontend-robusthed (Tasks 11–16)
- **Fase D** — Aktivér sovende backend-features (Tasks 17–24)
- **Fase E** — Manglende Discord-features (Tasks 25–36)
- **Fase F** — Beyond Discord (Tasks 37–39)
- **Fase G** — QA-gates og parity-rapport (Tasks 40–42)

---

## Fase A — Kritiske fixes

### Task 1: Implementér rigtigt OAuth2 token endpoint
- **Fil(er):** `crates/opencorde-api/src/routes/oauth2/token.rs`, `authorize.rs`, evt. ny `grants.rs`; tests i samme modul.
- **Hvad:** Erstat `"mock_token_placeholder"` (token.rs:27–29) med ægte authorization-code flow: validér client_id + client_secret (Argon2id-verify mod `oauth_client_secrets`, migration 074), validér engangs-authorization-code med expiry + redirect_uri-match, udsted access token (JWT med scope claims) + refresh token, understøt `grant_type=authorization_code|refresh_token`, og `POST /oauth2/revoke`. **Overlap:** Dette ER parity-plan Task 9 — koordinér via kanban så den ikke laves dobbelt.
- **Verificering:** Integrationstest: authorize → code → token → kald `/users/@me` med token → revoke → token afvist. Forkert secret/expired code/genbrugt code → 400/401. Grep bekræfter `mock_token_placeholder` er væk.
- **Commit:** `fix(oauth): implement real authorization code token exchange`

### Task 2: Validér invite-koder ved invite-only registrering
- **Fil(er):** `crates/opencorde-api/src/register.rs` (linje ~47), `crates/opencorde-db/src/repos/invite_repo.rs`.
- **Hvad:** Fjern TODO. I invite-only mode: slå koden op, kræv ikke-udløbet + uses < max_uses, inkrementér uses atomisk ved succes, returnér 403 med fejlkode `INVALID_INVITE` ellers.
- **Verificering:** Tests: gyldig kode → bruger oprettet + uses+1; udløbet/opbrugt/ukendt kode → 403; race-test (to samtidige registreringer på kode med 1 use tilbage → én vinder).
- **Commit:** `fix(auth): enforce invite code validation in invite-only mode`

### Task 3: 2FA recovery codes
- **Fil(er):** Ny migration `079_totp_recovery_codes.sql`; `crates/opencorde-api/src/routes/auth/` (totp-modulet); `client/src/lib/components/settings/TwoFactorSetup.svelte`.
- **Hvad:** Ved 2FA-aktivering genereres 10 engangskoder (vis én gang, gem kun Argon2id-hash). Login-flow accepterer recovery code i stedet for TOTP og forbruger den. Endpoint til regenerering (kræver password + TOTP). UI: vis koder med "gem dem nu"-advarsel, vis antal resterende.
- **Kode:**
  ```sql
  CREATE TABLE totp_recovery_codes (
    id BIGINT PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_hash TEXT NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );
  ```
- **Verificering:** Test: recovery-login forbruger koden (used_at sat, genbrug → 401); regenerering invaliderer gamle. Playwright: setup viser 10 koder.
- **Commit:** `feat(auth): add 2FA recovery codes`

### Task 4: Skift-password-mens-logget-ind
- **Fil(er):** `crates/opencorde-api/src/routes/users/` (ny `password.rs`); `client/src/routes/settings/` (Account-sektion).
- **Hvad:** `PATCH /api/v1/users/@me/password` med `{ current_password, new_password }`. Verificér nuværende password, kræv TOTP hvis aktiveret, hash nyt, revokér alle refresh tokens undtagen nuværende session (brug eksisterende JTI-infrastruktur, migration 041). UI-formular i settings med styrkeindikator (genbrug register-validering).
- **Verificering:** Test: forkert current → 403; succes → gammelt password afvist ved login, andre sessions revoked.
- **Commit:** `feat(account): change password while logged in`

### Task 5: Hardning af Tauri token-fallback
- **Fil(er):** `client/src/lib/stores/auth.ts` (linje ~60–94), `client/src/lib/api/client.ts`.
- **Hvad:** Keychain er primær på Tauri. Fallback til localStorage må kun ske på web-build; på Tauri skal keychain-fejl logges + toastes i stedet for stille plaintext-fallback. Dokumentér beslutningen i `decisions.md`.
- **Verificering:** Manuel Tauri-test: keychain bruges; simuleret keychain-fejl viser toast og gemmer ikke token plaintext.
- **Commit:** `fix(client): no silent plaintext token fallback on desktop`

---

## Fase B — Konventions- og kvalitetsgæld (backend)

### Task 6: Split backend-filer over 300 linjer (9 filer)
- **Fil(er):** `routes/messages/send_list.rs` (680→`send.rs`+`list.rs`), `routes/roles.rs` (605→`roles/`-modul), `routes/admin/upgrade.rs` (602→`upgrade.rs`+`upgrade_resolve.rs`), `routes/admin/jobs.rs` (596), `routes/admin/branding.rs` (557), `routes/dms/handlers.rs` (546), `routes/admin/backup.rs` (536), `routes/admin/storage.rs` (445), `routes/voice/handlers.rs` (421).
- **Hvad:** Rene mekaniske splits efter ansvar — ingen logikændringer. Én fil pr. commit eller pr. modul. Opdatér INDEX.md-filer.
- **Verificering:** `cargo test --workspace` grøn efter hvert split; `find crates -name '*.rs' -not -path '*/target/*' | xargs wc -l | awk '$1>300'` viser kun migrations/genereret.
- **Commit:** `refactor(api): split <fil> to honor 300-line rule` (én pr. fil)

### Task 7: Workspace clippy-lints
- **Fil(er):** Rod-`Cargo.toml` (`[workspace.lints]`), alle crate-`Cargo.toml` (`lints.workspace = true`).
- **Hvad:** Kodificér quality-gate-kravet: `clippy::all = "deny"`, overvej `clippy::unwrap_used = "warn"` i api-craten. Fix de warnings der opstår. Gennemgå de 11 `#[allow(...)]`: slet ubrugte felter bag `#[allow(dead_code)]` eller begrund dem med kommentar.
- **Verificering:** `cargo clippy --workspace --all-targets -- -D warnings` grøn.
- **Commit:** `chore(lints): enforce workspace clippy configuration`

### Task 8: Fjern usikre unwraps i unfurl + livekit
- **Fil(er):** `crates/opencorde-api/src/routes/unfurl.rs` (7 stk), `crates/opencorde-api/src/routes/voice/livekit.rs` (5 stk).
- **Hvad:** Erstat med `?`/`ok_or(ApiError::...)`. Unfurl-fejl må aldrig 500'e en message-send; LiveKit-fejl skal returnere meningsfuld fejl til klienten.
- **Verificering:** Tests med malformet HTML/utilgængelig URL (unfurl) og manglende LiveKit-config → pæne fejl, ingen panic.
- **Commit:** `fix(api): replace panicking unwraps with error propagation`

### Task 9: Tests til tynde repos
- **Fil(er):** `crates/opencorde-db/src/repos/dm_repo.rs` (468 linjer/3 tests), `bot_token_repo`, `interaction_repo`, `embed_repo`, `integration_log_repo`.
- **Hvad:** Dæk CRUD + edge cases (token-hash verify/revoke, interaction engangs-token + expiry, embed position-ordering, dm blocked-user-deny). Følg eksisterende testcontainers-mønster.
- **Verificering:** `cargo test -p opencorde-db` — hver nævnt repo har ≥8 tests.
- **Commit:** `test(db): cover dm and platform repos`

### Task 10: Integration-log API-ruter
- **Fil(er):** Ny `crates/opencorde-api/src/routes/integration_logs.rs`; `routes/mod.rs`.
- **Hvad:** Migration 078 + repo findes (commit 0ed1275) men ingen ruter. Tilføj `GET /api/v1/servers/{server_id}/integration-logs` (paginated, filter på application_id/action_type), gated på `server:VIEW_AUDIT_LOG`. **Overlap:** parity-plan Task 29 — denne task er kun API-laget; UI-panelet ligger der.
- **Verificering:** Test: webhook-execute + command-run skriver rækker; non-admin → 403.
- **Commit:** `feat(integrations): expose integration logs API`

---

## Fase C — Frontend-robusthed

### Task 11: Surface alle slugte fejl via toasts
- **Fil(er):** `client/src/lib/stores/auth.ts` (4× `catch {}`), `voice.ts` (9× noop), `messages.ts` (decryption-placeholder), `UserProfilePopover.svelte:78`, `ChannelSettingsModal.svelte`.
- **Hvad:** Gennemgå hver catch: (a) reelt forventet/benign → behold men tilføj `console.debug` med kontekst; (b) brugerpåvirkende (voice-join fejler, token-refresh fejler, decryption fejler) → `toastError()` fra `toasts.svelte.ts` med handlingsanvisning. Decryption-fejl: behold placeholder-tekst MEN tilføj én samlet toast pr. kanal, ikke pr. besked.
- **Verificering:** Grep: ingen tomme catch-blokke uden kommentar+debug-log. Playwright: dræb API'et midt i voice-join → toast vises.
- **Commit:** `fix(client): surface swallowed errors through toast system`

### Task 12: Færdiggør Svelte 5 store-migrering
- **Fil(er):** 25 legacy stores i `client/src/lib/stores/*.ts` → `.svelte.ts` med runes. Rækkefølge: blade først (theme, typing, drafts, emojis), kerne sidst (auth, messages, channels, servers, voice).
- **Hvad:** Én store pr. commit: konvertér `writable/derived` → `$state/$derived`, opdatér alle forbrugende komponenter, behold offentligt API-navn. Stop og rapportér hvis en konvertering kræver komponent-omskrivning >50 linjer.
- **Verificering:** `pnpm lint` + `pnpm check` grøn pr. commit; Playwright-smoke (login, send besked, skift kanal) efter hver kernestore.
- **Commit:** `refactor(stores): migrate <navn> to Svelte 5 runes` (én pr. store)

### Task 13: Virtualisér MessageList + lazy-load historik
- **Fil(er):** `client/src/lib/components/chat/MessageList.svelte` (479 linjer — splittes samtidig, jf. Task 14), ny `VirtualMessageWindow.svelte`.
- **Hvad:** Virtuelt vindue (render kun synlige beskeder + buffer; egen implementering med IntersectionObserver eller `@tanstack/virtual` — tilføj dependency med WHY-kommentar). Scroll-op henter ældre beskeder via eksisterende cursor-pagination. Bevar scroll-position ved prepend, autoscroll kun når brugeren er i bunden.
- **Verificering:** Seed 1.000 beskeder i testkanal; DOM-node-count holdes < 100; scroll-op loader ældre sider; Playwright: ny besked autoscroller kun fra bunden.
- **Commit:** `perf(chat): virtualize message list with lazy history`

### Task 14: Split frontend-filer over 300 linjer (8 filer)
- **Fil(er):** `StatusPicker.svelte` (535→Status/CustomStatus/Activity-sektioner), `MessageList.svelte` (479, sammen med Task 13), `admin/BrandingPanel.svelte` (411), `FederationPanel.svelte` (387), `JobsPanel.svelte` (378), `MessageInput.svelte` (335→input + AttachmentTray + autocomplete-wiring), `UpgradePanel.svelte` (319), `WaveformPlayer.svelte` (309).
- **Hvad:** Rene komponent-udtræk, ingen adfærdsændring.
- **Verificering:** `pnpm check` grøn; `find client/src -name '*.svelte' | xargs wc -l | awk '$1>300'` tom; Playwright-smoke.
- **Commit:** `refactor(client): split <fil> to honor 300-line rule`

### Task 15: Færdiggør stub-paneler — Sessions + AuthorizedApps
- **Fil(er):** `client/src/lib/components/settings/SessionsPanel.svelte`, `AuthorizedAppsPanel.svelte`; ny backend `crates/opencorde-api/src/routes/users/sessions.rs`.
- **Hvad:** Sessions: `GET /api/v1/users/@me/sessions` (fra refresh_tokens/JTI-tabellen, migration 041: enhed/IP/sidst brugt) + `DELETE .../sessions/{id}` + `DELETE .../sessions` (alle andre). UI: liste med "Log ud"-knapper, nuværende session markeret. AuthorizedApps: **overlap** parity-plan Task 11 — implementér KUN hvis Task 11 ikke allerede er i gang (check kanban); ellers skip her.
- **Verificering:** Revoked session kan ikke refreshe; Playwright: log ind fra to klienter, revoke den ene, den anden virker stadig.
- **Commit:** `feat(settings): session management panel`

### Task 16: Færdiggør/fjern resterende stubs
- **Fil(er):** `ActivityStatusPicker.svelte`, `AccountSwitcher.svelte`, `UserBadges.svelte`, `ProfileTab.svelte` (linje 185–191).
- **Hvad:** ActivityStatusPicker: wire mod presence-API (Task 18) — sæt activity_type/activity_name. AccountSwitcher + UserBadges: beslutning kræves — **spørg Marcin**: implementér eller fjern fra navigation (anbefaling: fjern AccountSwitcher fra nav til multi-account er et reelt behov; UserBadges venter på badge-data).
- **Verificering:** Ingen "coming soon"-paneler synlige i settings-navigationen.
- **Commit:** `feat(settings): wire activity status; remove dead stubs`

---

## Fase D — Aktivér sovende backend-features

### Task 17: Polls API + UI
- **Fil(er):** Ny `crates/opencorde-api/src/routes/messages/polls.rs`; `client/src/lib/components/chat/PollComposer.svelte` + `PollDisplay.svelte`; `MessageInput`/`MessageList`-wiring; WS-event `PollVote`.
- **Hvad:** Migration 069 (JSONB `poll` på messages) findes. Routes: opret poll (del af send_message payload), `PUT /messages/{id}/poll/votes/{answer_id}`, `DELETE` (fjern stemme), `POST /messages/{id}/poll/close` (kun author/MANAGE_MESSAGES), auto-close ved expires_at. UI: composer (spørgsmål, 2–10 svar, multiselect, varighed), resultatvisning med procentbarer, realtime via WS.
- **Verificering:** API-test: stem/skift/luk; multiselect-regler håndhæves. Playwright to-klient: stemme synkroniserer realtime.
- **Commit:** `feat(polls): poll voting API and UI`

### Task 18: Presence/aktivitet API + visning
- **Fil(er):** `crates/opencorde-api/src/routes/users/presence.rs` (udvid); `client/src/lib/stores/presence.ts`; `MemberList`/`UserProfilePopover`-komponenter; `StatusPicker.svelte`.
- **Hvad:** Migration 065-kolonner (custom_status_text/expires, activity_type/name) findes. `PATCH /users/@me/presence` sætter dem; WS `PresenceUpdate` broadcaster til fælles servere; expiry ryddes ved opslag. Visning: custom status i medlemsliste + popover ("Spiller X", "Lytter til Y").
- **Verificering:** To-klient: klient A sætter status → klient B ser den uden refresh; udløbet status forsvinder.
- **Commit:** `feat(presence): custom status and activity end-to-end`

### Task 19: Scheduled messages UI
- **Fil(er):** `client/src/lib/components/chat/` ny `ScheduleMessageModal.svelte` + `ScheduledMessagesList.svelte`; `MessageInput`-menu.
- **Hvad:** Backend (migration 072 + routes) findes. UI: "Send senere" i input-menuen (dato/tid-picker), liste over planlagte beskeder pr. kanal med annullér/redigér.
- **Verificering:** Playwright: planlæg 1 min frem → besked dukker op; annullér → gør ikke.
- **Commit:** `feat(chat): scheduled messages UI`

### Task 20: Message-request inbox UI
- **Fil(er):** `client/src/routes/@me/+page.svelte` (ny tab), ny `MessageRequestsPanel.svelte`; `client/src/lib/stores/dms.ts`.
- **Hvad:** Backend (migration 070: pending/accepted/ignored/spam) findes. UI: "Anmodninger"-tab i DM-hub med accept/ignorér/spam-knapper og preview af første besked; badge-count.
- **Verificering:** Playwright to brugere uden venskab: B's DM lander i A's requests; accept flytter til DMs; spam skjuler.
- **Commit:** `feat(dms): message request inbox UI`

### Task 21: Message components API + renderer
- **Hvad:** **Overlap:** Dette er parity-plan Tasks 24–25 (components-delen). Eksekvér der, ikke her. Denne task er kun en kanban-krydsreference så intet falder mellem to planer.
- **Verificering:** Kanban-link mellem de to planers tasks findes.

### Task 22: Granulære notifikationsindstillinger UI
- **Fil(er):** `client/src/lib/components/settings/NotificationsTab.svelte` (udvid), ny `ServerNotificationSettings.svelte` (i server-kontekstmenu), `ChannelNotificationSettings.svelte`; `client/src/lib/stores/notificationSettings.ts`.
- **Hvad:** Backend findes (migration 066: level ALL/MENTIONS/NOTHING, mute_until, suppress everyone/here/roles; plus per-kanal i users-ruterne). UI: højreklik server → mute (15m/1t/8t/24t/∞) + niveau; kanal-niveau tilsvarende; suppress-toggles. Klienten skal respektere indstillingerne ved afspilning af lyd/desktop-notifikationer og unread-badges.
- **Verificering:** Mutet server giver ingen badge/lyd ved ny besked (Playwright + manuel); mute_until udløber korrekt.
- **Commit:** `feat(notifications): granular mute and suppression UI`

### Task 23: Recordings management UI
- **Fil(er):** Ny `client/src/lib/components/voice/RecordingsPanel.svelte`; wiring i kanal-/serverindstillinger.
- **Hvad:** Routes (start/stop/list, migration 037) findes. UI: liste med dato/varighed/størrelse, afspil (genbrug `WaveformPlayer`), download, slet (gated `MANAGE_CHANNELS`).
- **Verificering:** Optag 10 sek voice → vises i panel → kan afspilles/slettes.
- **Commit:** `feat(voice): recordings management panel`

### Task 24: Data export UI
- **Fil(er):** `client/src/routes/settings/` Privacy-sektion.
- **Hvad:** Route findes. Knap "Anmod om dataeksport" → poll status → download-link. GDPR-venlig tekst.
- **Verificering:** Eksport downloader gyldig JSON/zip med egne data.
- **Commit:** `feat(privacy): data export UI`

---

## Fase E — Manglende Discord-features

### Task 25: Group DMs
- **Fil(er):** Ny migration `080_group_dms.sql`; `crates/opencorde-db/src/repos/dm_repo.rs` (udvid — splits hvis >300 linjer); `routes/dms/`; `client` DM-hub + `GroupDmHeader.svelte`.
- **Hvad:** Inspicér migration 014 (dm_channels) først. Udvid: `is_group BOOLEAN`, `name VARCHAR(100)`, `owner_id`, `icon_url`, og `dm_members(dm_id, user_id, joined_at)` join-tabel (1:1-DMs migreres logisk ind eller håndteres ved is_group=false). Routes: opret gruppe (2–10 venner), tilføj/fjern medlem (owner eller self-leave), omdøb, skift ejer ved leave. WS-events for medlemsændringer. UI: opret fra venneliste (multi-select), medlemsliste i header.
- **Verificering:** API-tests: kun venner kan tilføjes; max 10; leave→ejerskifte. Playwright tre-bruger: gruppe oprettes, besked ses af alle, fjernet medlem mister adgang (historik bevares server-side).
- **Commit:** `feat(dms): group direct messages`

### Task 26: Image lightbox
- **Fil(er):** Ny `client/src/lib/components/chat/ImageLightbox.svelte`; `MessageList`/attachment-rendering.
- **Hvad:** Klik på billede → fuldskærmsoverlay med zoom (scroll/pinch), pil-navigation mellem billeder i kanalen, Esc lukker, download-knap, `role="dialog"` + fokusfælde.
- **Verificering:** Playwright: åbn/navigér/luk med tastatur.
- **Commit:** `feat(chat): image lightbox viewer`

### Task 27: Paste-image upload
- **Fil(er):** `client/src/lib/components/chat/MessageInput.svelte` (efter Task 14-split: AttachmentTray).
- **Hvad:** `paste`-handler: clipboard-billeder → samme upload-flow som filvælger (preview, fjern-knap, størrelses-/MIME-validering genbruges).
- **Verificering:** Playwright med clipboard-permission: paste PNG → preview → send → vises i kanal.
- **Commit:** `feat(chat): paste image to upload`

### Task 28: GIF picker (pluggable provider)
- **Fil(er):** Ny `crates/opencorde-api/src/routes/gifs.rs` (proxy); admin-config (`routes/admin/branding.rs`-mønster) for provider+API-nøgle; ny `client/src/lib/components/chat/GifPicker.svelte`.
- **Hvad:** Selvhostet hensyn: klienten må ikke kalde Tenor/Giphy direkte (IP-læk). Backend-proxy `GET /api/v1/gifs/search?q=` med provider-interface (Tenor først), cache i Redis (1t), API-nøgle kun i server-env. Feature er slukket hvis ingen nøgle konfigureret (picker skjult). UI: GIF-knap ved emoji-pickeren, grid med søgning, send som attachment-URL.
- **Verificering:** Med nøgle: søg/send GIF virker; uden nøgle: knap skjult; ingen klient-kald til tredjepart (network-inspektion).
- **Commit:** `feat(chat): GIF picker with server-side provider proxy`

### Task 29: Stickers (server sticker packs)
- **Fil(er):** Ny migration `081_stickers.sql`; `routes/servers/stickers.rs`; `client` StickerPicker + settings-panel (følg emojis-mønsteret: migration 027, `EmojisPanel.svelte`).
- **Hvad:** `server_stickers(id, server_id, name, description, file_id, created_by)` — upload via MinIO som emojis (PNG/APNG/WebP, max 512KB). Besked-payload: `sticker_ids[]`. Picker-tab ved siden af emoji/GIF. Gated `MANAGE_EMOJIS`-permission (omdøb evt. til `MANAGE_EXPRESSIONS` — kræver beslutning, log i decisions.md).
- **Verificering:** Upload→send→render; slettet sticker viser pænt fallback i gamle beskeder.
- **Commit:** `feat(stickers): server sticker packs`

### Task 30: Jump-to-date + søgefiltre
- **Fil(er):** `crates/opencorde-api/src/routes/messages/` (ny `around.rs`); `crates/opencorde-search/` (filter-parsing); `client` SearchModal + ny `JumpToDateModal.svelte`.
- **Hvad:** (a) `GET /channels/{id}/messages?around={snowflake}` (snowflake kan konstrueres fra dato) + UI-kalender der hopper i historikken (kræver Task 13-virtualisering). (b) Søgefiltre `from:@user`, `has:link|image|file`, `before:/after:/during:` parsed server-side i Tantivy-laget; filterchips i SearchModal.
- **Verificering:** Hop til dato 3 måneder tilbage lander korrekt; `from:user has:image` returnerer kun matchende.
- **Commit:** `feat(search): jump-to-date and search filters`

### Task 31: Role icons
- **Fil(er):** Ny migration `082_role_icons.sql` (`roles.icon_url VARCHAR`, `roles.unicode_emoji VARCHAR(32)`); `routes/`-roles (efter Task 6-split); `RolesPanel.svelte`, medlemsliste-rendering.
- **Hvad:** Upload (MinIO, 256KB, samme validering som emojis) eller unicode-emoji pr. rolle. Vis ikon ved højeste hoisted rolle i medlemsliste + popover.
- **Verificering:** Ikon sat → vises hos andre klienter realtime (RoleUpdate-event findes allerede).
- **Commit:** `feat(roles): role icons`

### Task 32: Server templates
- **Fil(er):** Ny migration `083_server_templates.sql`; `routes/servers/templates.rs`; `client` opret-server-flow.
- **Hvad:** `POST /servers/{id}/templates` serialiserer struktur (kanaler, kategorier, roller m. permissions, overwrites, automod-regler, onboarding — IKKE beskeder/medlemmer) til versioneret JSON. `POST /servers?template={code}` opretter fra template. Offentlig template-kode (8 tegn). UI: "Opret fra template" i serveroprettelse + template-administration i serverindstillinger.
- **Verificering:** Template af testserver → ny server har identiske kanaler/roller/overwrites (diff-script i `scripts/`).
- **Commit:** `feat(servers): server templates`

### Task 33: Vanity URLs
- **Fil(er):** Ny migration `084_vanity_urls.sql` (`servers.vanity_code VARCHAR(32) UNIQUE`); invite-resolve-ruten; `OverviewPanel.svelte`.
- **Hvad:** Server-ejer kan sætte vanity code (regex `[a-z0-9-]{3,32}`, reserveret-ords-liste: admin, api, app m.fl.). `/invite/{vanity}` resolver som almindelig invite uden expiry/uses. Gated `MANAGE_SERVER`.
- **Verificering:** Vanity-join virker; kollision → 409; reserveret ord → 400.
- **Commit:** `feat(servers): vanity invite URLs`

### Task 34: User notes
- **Fil(er):** Ny migration `085_user_notes.sql` (`user_notes(author_id, target_id, note VARCHAR(256), PK(author_id,target_id))`); `routes/users/notes.rs`; `UserProfilePopover.svelte`.
- **Hvad:** Privat note pr. bruger, kun synlig for author. `PUT/GET /users/{id}/note`. Tekstfelt i profil-popover, autosave med debounce.
- **Verificering:** Note gemmes/vises kun for author (anden bruger ser den ikke — API-test).
- **Commit:** `feat(users): private user notes`

### Task 35: Member screening / regler
- **Fil(er):** Ny migration `086_member_screening.sql`; `routes/servers/screening.rs`; `client` join-flow + `ModerationPanel.svelte`.
- **Hvad:** Server kan aktivere screening: regelsæt (markdown-liste) som nye medlemmer skal acceptere før de kan skrive (membership får `pending BOOLEAN` — pending members ser kanaler men kan ikke sende; håndhæves i send_message-permission-tjek, IKKE kun UI). Accept-modal ved join. Integrér med eksisterende onboarding (migration 046) så de ikke konflikter.
- **Verificering:** Pending medlem: send → 403; efter accept: send OK; audit-log-entry ved accept.
- **Commit:** `feat(moderation): member screening with rules acceptance`

### Task 36: Søgefilter-UI + quick-switcher polish (opsamling)
- **Fil(er):** `SearchModal.svelte`, `QuickSwitcher`-komponenten.
- **Hvad:** Wire Task 30-filtre som klikbare chips; quick-switcher (Ctrl+K) viser også gruppe-DMs (Task 25) og venner.
- **Verificering:** Playwright: filterchips ændrer resultater; Ctrl+K finder gruppe-DM.
- **Commit:** `feat(search): filter chips and switcher coverage`

---

## Fase F — Beyond Discord

### Task 37: E2EE end-to-end bevis
- **Fil(er):** Ny `scripts/e2ee_two_client_qa.py`; `reports/raw/e2ee-proof.json`.
- **Hvad:** E2EE (OpenMLS, migrations 031/032/047) er implementeret men uproven. To-klient-bevis: aktivér E2EE på kanal → begge klienter sender/læser → DB-inspektion bekræfter kun `enc:`-ciphertext på disk → tredje klient uden welcome kan IKKE læse → fil-attachment er AES-GCM-krypteret i MinIO. Det er differentiatoren Discord ikke har — den skal være bevist, ikke bare bygget.
- **Verificering:** Rapport med checks_failed=0; DB-dump-grep viser ingen plaintext i E2EE-kanal.
- **Commit:** `test(e2ee): prove two-client encrypted channel lifecycle`

### Task 38: Federation cross-instance bevis
- **Fil(er):** `scripts/federation_qa.py`; `reports/raw/federation-proof.json`; docker-compose-profil med to instanser.
- **Hvad:** Federation-ruter (migrations 044/045) er ubeviste. Spin to instanser op lokalt (compose), gennemfør identity-lookup + event-sync mellem dem, dokumentér hvad der reelt virker vs. skitseret. Resultatet kan være "delvist — disse 3 ting mangler": det er også et gyldigt outcome, log det i parity-rapporten.
- **Verificering:** Rapport med ærlig status pr. federation-feature.
- **Commit:** `test(federation): cross-instance capability proof`

### Task 39: Accessibility-pass
- **Fil(er):** Modaler/paneler på tværs af `client/src/lib/components/`.
- **Hvad:** Luk hullerne fra review: `aria-modal` + fokusfælde på ALLE modaler (genbrug mønsteret fra `KeyboardShortcutsHelp.svelte`), synlige fokusindikatorer (Tailwind ring), `aria-live` på toast-containeren, axe-core-scan i Playwright-suiten.
- **Verificering:** axe-scan: 0 critical/serious på login, chat, settings, server settings.
- **Commit:** `fix(a11y): modal focus management and axe compliance`

---

## Fase G — QA-gates og parity-rapport

### Task 40: Udvid API-smoke-suiten
- **Fil(er):** `scripts/` (følg `platform_api_qa.py`-mønsteret fra parity-planen); `reports/raw/features-api-proof.json`.
- **Hvad:** Scenarier for alt nyt: oauth-flow, invite-validering, recovery-code-login, password-change, polls, presence, group DM, stickers, vanity, screening, notes, søgefiltre, notifikations-mute.
- **Verificering:** checks_failed=0, ingen secrets i rapporten.
- **Commit:** `test(qa): feature API smoke suite`

### Task 41: Udvid Playwright-suiten
- **Fil(er):** `scripts/`; screenshots under `reports/parity-screenshots/features/`.
- **Hvad:** UI-bevis for: lightbox, paste-upload, GIF/sticker-picker, poll realtime, message-requests, scheduled messages, sessions-panel, notification-mute, group DM (tre brugere), jump-to-date. Desktop + mobil-viewport.
- **Verificering:** Screenshots + JSON-rapport, 0 fejl.
- **Commit:** `test(qa): feature UI proof suite`

### Task 42: Opdatér parity-rapport + ReadMeFirst + kanban
- **Fil(er):** `reports/discord-parity.json`, `ReadMeFirst.md`, `project-map.yaml`, INDEX.md-filer, kanban-board `opencorde`.
- **Hvad:** Flyt områder til proven KUN med live-evidens (jf. husreglen). Tilføj `feature_breakdown` for de nye områder. Opdatér ReadMeFirst "Current Project State". Synkronisér med parity-planens Task 39/40 så de to planer rapporterer i samme struktur.
- **Verificering:** `python3 -m json.tool reports/discord-parity.json`; live site verificeret før nogen "proven"-claims.
- **Commit:** `docs(parity): track feature completion`

---

## Eksekveringsrækkefølge

**Kritisk sti (seriel):** Task 1 → 2 → 3 → 4 (Fase A færdig før alt andet — sikkerhed først).

**Derefter parallelle spor:**
- **Spor 1 (backend-kvalitet):** Task 6 → 7 → 8 → 9 → 10
- **Spor 2 (frontend-kvalitet):** Task 11 → 14 → 13 → 12 (splits før virtualisering før store-migrering)
- **Spor 3 (features, efter spor 1+2 er ~halvvejs):** Task 17–24, derefter 25–36 i nummerorden (25/Group DMs og 30/søg er de største — giv dem egne subagents)
- **Spor 4 (beyond):** Task 37–39 kan køre når som helst (uafhængige)

**Afhængigheder:**
- Task 13 (virtualisering) før Task 30a (jump-to-date).
- Task 14 (MessageInput-split) før Task 27 (paste-upload).
- Task 25 (group DMs) før Task 36 (switcher-coverage).
- Task 1 koordineres med parity-planens Task 9; Task 15/21 krydsrefererer parity-planens Task 11/24–25 — check kanban FØR start så intet laves dobbelt.
- Fase G sidst.

---

## Acceptance: "100% and beyond"

Planen er færdig når:

1. `grep -r "mock_token_placeholder\|TODO: validate invite" crates/` er tom.
2. Ingen kildefil over 300 linjer (begge sider af stakken).
3. `cargo clippy --workspace -- -D warnings` og `pnpm lint && pnpm check` er grønne i CI.
4. Ingen tomme catch-blokke; alle brugerpåvirkende fejl toaster.
5. Alle 5 stub-paneler er implementeret eller fjernet fra navigation.
6. Polls, presence, scheduled messages, message-requests, recordings, data-export, notifikations-mute har fungerende UI bevist med Playwright.
7. Group DMs, lightbox, paste-upload, GIF, stickers, jump-to-date, søgefiltre, role icons, templates, vanity, notes, screening virker live.
8. E2EE- og federation-beviser er kørt og ærligt rapporteret.
9. `reports/discord-parity.json` opdateret med evidens; ingen "proven" uden live-verifikation.
10. Emma/platform-laget (den anden plan) er IKKE blokeret af noget i denne plan.
