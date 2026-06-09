# OpenCorde Discord Platform Parity — Strategisk Implementeringsplan

> **Til Hermes:** Eksekvér denne plan med subagent-driven-development. Brug OpenCorde skill, læs `ReadMeFirst.md`, og arbejd task-for-task. Ingen produktionsændringer uden Marcin's eksplicitte godkendelse.

**Mål:** Gør OpenCorde til en egentlig udviklerplatform på Discord-niveau: apps, bots, OAuth2, gateway intents, application commands/interactions, rich embeds, webhooks, auditability og Emma som dual-platform bot.

**Nuværende tilstand:** OpenCorde er allerede en stærk chat/voice/community-app med 176 routes, permission gates, messaging, LiveKit voice/stage, roles/overwrites, webhooks, slash command scaffolding, audit log, automod og Discord bridge. Men platform-laget er stadig overvejende shallow: bot identity, OAuth2 grants, gateway intents, interaction lifecycle, embed payloads, developer portal og Emma-as-real-bot mangler eller er kun skitseret.

**Ønsket tilstand:** OpenCorde skal fungere som en programmerbar platform. En udvikler skal kunne oprette en application i Developer Portal, få client ID/secret, oprette/rotere bot token, vælge privileged intents, installere appen på en server via OAuth2-lignende flow, registrere slash/user/message commands, modtage gateway events eller HTTP interactions, svare med public/ephemeral/deferred responses, sende rich embeds via bot/webhook, og se audit/integration logs. Emma skal bruge samme OpenCorde bot API som andre bots — ikke DB shortcuts.

**Arkitektur:**

```text
Developer Portal / Server Settings
        |
        v
+---------------------+        +---------------------+
| App Control Plane   |        | Governance Plane    |
| applications        |------->| audit_log           |
| bot_users           |        | integration_logs    |
| bot_tokens          |        | app trust/reviews   |
| oauth_clients       |        | automod events      |
| app_installs/grants |        +---------------------+
+----------+----------+
           |
           +---------------------+----------------------+
           |                     |                      |
           v                     v                      v
+---------------------+ +----------------------+ +----------------------+
| Bot Gateway Plane   | | Interaction Plane    | | Message/Webhook Plane|
| WS opcode protocol  | | app commands         | | messages.embeds      |
| intents             | | interaction tokens   | | webhook tokens/hash  |
| sessions/resume     | | callbacks/followups  | | edit/delete/history  |
| event filters       | | components/modals    | | allowed_mentions     |
+----------+----------+ +----------+-----------+ +----------+-----------+
           |                       |                        |
           +-----------------------+------------------------+
                                   |
                                   v
                       Emma OpenCorde Adapter
                       Discord adapter remains live
```

**Afhængigheder:**
- Repo: `/home/mb/opencorde`.
- Backend: Rust/Axum, SQLx/PostgreSQL, Redis for gateway/session/event replay, existing WS gateway.
- Frontend: SvelteKit routes/components/stores, existing server settings and chat components.
- Existing M7 kanban epic: `t_f6a3c226` Apps, Bots, Commands, and Webhooks.
- Existing M7.1 child: `t_6b4ede90` has review-required implementation for migration 064 + models/repos. It must be reviewed before downstream work.
- Emma: Red-DiscordBot on Thor (`emma_discord_bot`) remains Discord runtime; OpenCorde integration should be a separate Python adapter/library using OpenCorde bot HTTP/WS APIs.
- Secrets: bot tokens and client secrets must be stored hashed server-side and in `.env`/operator-local files only. No tokens in repo, reports, screenshots, kanban handoffs.

**Risici:**
- **Shallow parity risk:** Merely adding tables/routes is not platform parity. Acceptance must be live workflow proof: developer creates app -> installs bot -> bot connects -> command executes -> interaction response visible -> audit logs show it.
- **Security risk:** Bot tokens and webhook tokens are bearer credentials. Store only hashes, show once, support revoke/rotate, log usage, rate-limit creation/execution.
- **Gateway complexity:** Discord-like gateway needs opcodes, heartbeats, seq, resume, intents, event filtering and eventually sharding. Build minimal correct protocol first; do not bolt bots onto the user WS session model.
- **Permission mismatch:** App permissions are not identical to user permissions. Bot effective permission = server membership role perms + channel overwrites + app install grants + app channel restrictions + command permissions.
- **Interaction lifecycle bugs:** Ephemeral/deferred/followup responses require separate visibility semantics from normal messages. Do not implement ephemeral as “hidden CSS”; enforce server-side visibility.
- **Emma architecture risk:** Red-DiscordBot is Discord-specific. Treat Emma as business logic plus two platform adapters, not one bot trying to pretend OpenCorde is Discord.
- **Over-large files:** OpenCorde convention: no source file over 300 lines; split modules early.

**Impact ranking — 3 vigtigste ting først:**
1. **App identity/control plane**: applications, bot users, OAuth2 scopes/grants, developer portal, install model. Uden dette findes der ingen platform-økonomi, ingen safe bot ownership, ingen token lifecycle.
2. **Bot gateway + intents**: persistent bot connection with identify/heartbeat/resume/event filtering. Uden dette kan Emma og andre bots ikke “leve” i OpenCorde; de kan kun kalde REST.
3. **Interactions + rich message/webhook substrate**: application commands, interaction callback lifecycle, embeds/components/ephemeral/deferred responses. Det er her Discord føles som en platform for brugeren.

---

## Hvad Discord gør til en PLATFORM — ikke bare chat

Discord's platform er ikke “slash commands”. Det er et fuldt økosystem med fem lag:

1. **Identity & trust:** Applications har ejere, teams, client IDs, secrets, bot users, public/private metadata, verification og install grants.
2. **Authorization:** OAuth2 scopes, permission bitfields, install-time permission preview, privileged intents, app command permissions.
3. **Event delivery:** Gateway opcodes, heartbeats, sequence numbers, resume, intents, rate limits og shardability.
4. **User-facing interaction model:** Slash/user/message commands, autocomplete, buttons, selects, modals, ephemeral/deferred/followup responses.
5. **Governance:** Audit log, integration logs, AutoMod, app discovery/trust, webhook history, token rotation, abuse prevention.

OpenCorde har stærke community primitives. Det manglende dybe lag er programmability: sikre API-kontrakter, stable SDK-like behavior, developer UX og operational traceability.

---

## Genbrug fra eksisterende OpenCorde

**Kan genbruges direkte eller med moderate udvidelser:**
- `permissions.rs` + 30 permission types: basis for bot permissions, command permissions og install preview.
- Permission gates from `reports/raw/route-inventory.json`: existing `channel:MANAGE_WEBHOOKS`, `channel:USE_APPLICATION_COMMANDS`, `server:VIEW_AUDIT_LOG`, etc.
- Existing users/server_members/roles/channel overwrites: bot users can be real users with `is_bot=true` plus membership/roles.
- `GatewayEvent` enum + WS dispatch: foundation for bot event payloads, but must be wrapped in a Discord-like opcode envelope for bots.
- Messaging pipeline: message rows, attachments, reactions, pins, replies, realtime `MessageCreate/Update/Delete` can be reused for bot/webhook posts.
- Existing webhook route/UI: can evolve into Discord-style webhooks.
- Existing slash command route/UI: can evolve into application commands, but current model is too simple.
- Existing `audit_log` table/routes and `log_mod_action`: foundation for broad audit/integration log coverage.
- Existing automod keyword rules: foundation for deeper AutoMod events/actions.
- Discord bridge/ghost users: useful for federation/bridge and Emma dogfooding, but not a substitute for first-class OpenCorde bots.

**Kan ikke genbruges uden redesign:**
- Current slash commands: server-scoped `handler_url` callbacks create a response message authored by invoking user. Discord-style commands are app-owned, option-schema based, interaction-token based, support autocomplete, defer, ephemeral and followups.
- Current webhooks: route is `/api/v1/webhooks/{token}/execute`, token appears plaintext, only `content`/`username`, and message author is creator user. Needs Discord-like `{webhook_id}/{token}` identity, token hashing, bot/webhook author semantics, embeds, attachments, edit/delete/history.
- Current user gateway: only `Identify { token }`, `Heartbeat`, `Ready`, no bot auth, intents, opcode envelope, seq/resume/session buffers.
- Message model: current core `Message` has no embeds, components, flags, webhook_id, application_id, interaction metadata or ephemeral visibility.
- Developer Portal: no first-class frontend route; `AuthorizedAppsPanel` is a stub.

---

## Gap matrix: Discord Developer Portal vs OpenCorde

| Area | Discord depth | OpenCorde now | Gap severity | First build |
|---|---|---|---:|---|
| Applications | App registration, client ID/secret, bot user, team ownership | Migration 064/models/repos partially present; no API/portal | Critical | App CRUD + owner/developer portal |
| Bot tokens | Create/reset token, privileged intents, invite/install links | None | Critical | Hash-backed token lifecycle |
| OAuth2 | Scopes, grants, redirects, authorized apps | Scope seed only in migration 064; no routes | Critical | OAuth authorize/callback/token/revoke |
| Gateway intents | Named event buckets, privileged review, identify/resume | User WS only | Critical | Bot WS protocol and intent filtering |
| Commands | CHAT_INPUT/USER/MESSAGE/ENTRY_POINT, options, choices, autocomplete | simple server slash command with handler_url | Critical | Application command schema + registry |
| Interactions | Immediate/deferred/ephemeral/followup, modals/components | none beyond simple interact | Critical | Interaction rows/tokens/callback API |
| Embeds | title/description/url/timestamp/color/footer/image/thumbnail/video/provider/author/fields | no message embed model | High | `message_embeds` / embedded JSON payload + renderer |
| Webhooks | execute/edit/delete messages, embeds, avatar/username override, wait/thread | basic execute/list/create/delete | High | webhook identity/token/hash/embeds/history |
| Audit log | typed action categories + changes/options | table + route + some mod actions | High | typed audit emitter for all mutating operations |
| AutoMod | triggers/actions, keyword/spam/mention filters, events | keyword contains only | Medium | mention spam + rate spam + action logs |
| Discovery | App/server discovery + trust/reviews | server discovery exists; no app directory | Medium | app directory entries/reviews/trust |
| Forum channels | posts/tags/following/pinning | routes exist; proof varies | Medium | tag/pin/follow events for bot intents |
| Rich presence | Activities/assets/party/timestamps | status presence scaffold only | Medium | activity model + Gateway presence events |

---

# Faser

1. Foundation review and platform contract
2. Developer Portal and application control plane
3. OAuth2, installs, scopes and authorized apps
4. Bot tokens, gateway intents and event delivery
5. Application commands and interaction lifecycle
6. Rich embeds, components and webhook parity
7. Audit log, integration logs and governance
8. Emma dual-platform bot
9. QA, live proof and kanban alignment

---

# Tasks

## Fase 1 — Foundation review and platform contract

### Task 1: Review and unblock M7.1 app model foundation
- **Fil(er):**
  - `crates/opencorde-db/migrations/064_apps_and_bots.sql`
  - `crates/opencorde-core/src/models/application.rs`
  - `crates/opencorde-core/src/models/app_install.rs`
  - `crates/opencorde-db/src/repos/app_repo.rs`
  - `crates/opencorde-db/src/repos/bot_user_repo.rs`
- **Hvad:** Review existing `t_6b4ede90` output. Decide whether `applications.name` should be globally unique or owner-scoped, whether `redirect_uris JSONB` stays, whether `app_installs.scopes TEXT[]` is enough, and whether bot user relationship should be `users.application_id` + `bot_users.id=user_id`.
- **Kode:** Keep migration 064 pattern, but add explicit unique constraint for bot user per app if missing:
  ```sql
  CREATE UNIQUE INDEX IF NOT EXISTS idx_bot_users_application_unique
    ON bot_users (application_id);
  ```
- **Verificering:**
  - `cargo fmt --check`
  - `cargo test -p opencorde-core application app_install oauth bot --quiet`
  - `cargo test -p opencorde-db app --quiet`
  - Kanban `t_6b4ede90` can move from blocked/review-required to done.
- **Commit:** `review(apps): accept application foundation schema`

### Task 2: Define OpenCorde public developer API contract
- **Fil(er):**
  - Create `docs/developer-platform/api-contract.md`
  - Modify `docs/INDEX.md` if present
- **Hvad:** Write the API contract before coding endpoints. Include object schemas for Application, BotUser, BotToken, OAuthScope, AppInstall, ApplicationCommand, Interaction, InteractionResponse, MessageEmbed, Webhook, AuditLogEntry. State compatibility target: Discord-like semantics, OpenCorde-specific paths under `/api/v1`.
- **Kode:** Include this contract skeleton:
  ```markdown
  ## Compatibility rule
  OpenCorde does not need byte-for-byte Discord API compatibility, but every Discord concept must map to a stable OpenCorde object, permission, event and audit action.

  ## Bot token rule
  Plain token is returned exactly once. Server stores Argon2id hash plus non-secret prefix.
  ```
- **Verificering:** Contract covers all 12 platform areas from the gap matrix.
- **Commit:** `docs(platform): define developer API contract`

### Task 3: Add platform route inventory baseline
- **Fil(er):**
  - Modify `scripts/route_inventory.py`
  - Create `reports/raw/platform-route-baseline.json`
- **Hvad:** Extend route inventory output to tag platform routes: applications, oauth, bot_tokens, gateway, interactions, webhooks, embeds, audit, integration_logs.
- **Kode:** Add a category map:
  ```python
  PLATFORM_ROUTE_KEYWORDS = {
      "applications": ["/applications", "app_install"],
      "oauth": ["/oauth2", "authorize", "token"],
      "bots": ["/bot", "bot_tokens", "gateway"],
      "interactions": ["/interactions", "/commands", "/interact"],
      "webhooks": ["/webhooks"],
      "governance": ["audit-log", "integration-logs", "automod"],
  }
  ```
- **Verificering:** `python3 scripts/route_inventory.py` produces `platform_categories` and still counts 176 existing routes before new work.
- **Commit:** `test(platform): add developer route baseline`

---

## Fase 2 — Developer Portal and application control plane

### Task 4: Create application API routes
- **Fil(er):**
  - Create `crates/opencorde-api/src/routes/applications/mod.rs`
  - Create `create.rs`, `list.rs`, `get.rs`, `update.rs`, `delete.rs`, `types.rs`
  - Modify `crates/opencorde-api/src/routes/mod.rs`
- **Hvad:** Add first-class app CRUD. Owner can create/list/update/delete own apps. Public can view public app profile. Do not expose secrets.
- **Kode:** Routes:
  ```rust
  Router::new()
      .route("/api/v1/applications", post(create_application))
      .route("/api/v1/applications/@me", get(list_my_applications))
      .route("/api/v1/applications/public", get(list_public_applications))
      .route("/api/v1/applications/{id}", get(get_application).patch(update_application).delete(delete_application))
  ```
- **Verificering:** API tests prove non-owner cannot update/delete; public list excludes private apps.
- **Commit:** `feat(apps): add application CRUD API`

### Task 5: Add bot user creation endpoint
- **Fil(er):**
  - Create `crates/opencorde-api/src/routes/applications/bot.rs`
  - Modify `crates/opencorde-db/src/repos/bot_user_repo.rs`
  - Modify `crates/opencorde-core/src/models/user.rs` if response needs `is_bot`
- **Hvad:** Owner can create a bot user attached to an application. Bot user is a real `users` row with `is_bot=true`, no password login, no refresh tokens.
- **Kode:** Route:
  ```rust
  POST /api/v1/applications/{id}/bot
  Body: { "username": "Emma", "avatar_url": null }
  Response: { "id", "application_id", "username", "avatar_url", "created_at" }
  ```
- **Verificering:** Bot user cannot login through `/auth/login`; bot appears as bot in member list once installed.
- **Commit:** `feat(apps): create bot users for applications`

### Task 6: Build Developer Portal frontend route
- **Fil(er):**
  - Create `client/src/routes/developers/+page.svelte`
  - Create `client/src/routes/developers/applications/[appId]/+page.svelte`
  - Create `client/src/lib/stores/applications.svelte.ts`
  - Create `client/src/lib/components/developers/ApplicationForm.svelte`
  - Create `client/src/lib/components/developers/ApplicationSettings.svelte`
- **Hvad:** Dedicated Developer Portal, not buried inside server settings. List apps, create app, edit metadata, create bot, preview install URL, manage tokens later.
- **Kode:** Store methods:
  ```ts
  export const applicationsStore = {
    async fetchMine() { return api.get('/applications/@me'); },
    async create(input) { return api.post('/applications', input); },
    async update(id, input) { return api.patch(`/applications/${id}`, input); },
    async createBot(id, input) { return api.post(`/applications/${id}/bot`, input); }
  };
  ```
- **Verificering:** Playwright: login -> `/developers` -> create app -> app appears -> create bot -> bot panel appears.
- **Commit:** `feat(portal): add developer portal application UI`

### Task 7: Add application ownership/team developers
- **Fil(er):**
  - Create migration `0XX_application_developers.sql`
  - Create repo functions in `app_repo.rs`
  - Extend API routes in `applications/developers.rs`
- **Hvad:** Discord has teams; OpenCorde minimum should support app owner + developer collaborators. Owners can add/remove developers; developers can manage commands but not delete app or rotate owner secrets unless granted.
- **Kode:**
  ```sql
  CREATE TABLE application_developers (
    application_id BIGINT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(24) NOT NULL DEFAULT 'developer',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (application_id, user_id)
  );
  ```
- **Verificering:** Non-owner developer can update command definitions; cannot delete app.
- **Commit:** `feat(apps): add application developer collaborators`

---

## Fase 3 — OAuth2, installs, scopes and authorized apps

### Task 8: Add OAuth2 client secret lifecycle
- **Fil(er):**
  - Create migration `0XX_oauth_clients.sql`
  - Create `crates/opencorde-api/src/routes/oauth2/clients.rs`
  - Modify `applications` API response
- **Hvad:** Add client ID (= app snowflake) and client secret creation/rotation. Store `secret_hash`, display only once.
- **Kode:**
  ```sql
  CREATE TABLE oauth_client_secrets (
    id BIGINT PRIMARY KEY,
    application_id BIGINT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    secret_prefix VARCHAR(12) NOT NULL,
    secret_hash TEXT NOT NULL,
    created_by BIGINT NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ
  );
  ```
- **Verificering:** Created secret returned once; list shows only prefix; revoked secret fails token exchange.
- **Commit:** `feat(oauth): add client secret lifecycle`

### Task 9: Add OAuth2 authorize and token endpoints
- **Fil(er):**
  - Create `crates/opencorde-api/src/routes/oauth2/mod.rs`
  - Create `authorize.rs`, `token.rs`, `types.rs`, `grants.rs`
  - Modify `routes/mod.rs`
- **Hvad:** Implement minimum OAuth2 authorization code flow for `identify`, `guilds`, `bot`, `applications.commands`, `webhook.incoming`. Use HttpOnly user auth session to approve grants.
- **Kode:** Routes:
  ```rust
  GET  /api/v1/oauth2/authorize?client_id=&redirect_uri=&scope=&state=&server_id=
  POST /api/v1/oauth2/authorize
  POST /api/v1/oauth2/token
  POST /api/v1/oauth2/revoke
  ```
- **Verificering:** OAuth smoke: user authorizes app -> code exchanged -> access token can call `/users/@me` and `/users/@me/guilds` equivalent.
- **Commit:** `feat(oauth): implement authorization code flow`

### Task 10: Add app install grants and permission preview
- **Fil(er):**
  - Create `crates/opencorde-api/src/routes/applications/install.rs`
  - Modify `app_installs` schema if needed
  - Create `client/src/lib/components/developers/InstallPreview.svelte`
- **Hvad:** Install app to server with explicit scopes and permission bits. Requires `MANAGE_SERVER` or future `MANAGE_APPS`. Adds bot user to `server_members` and assigns selected role/permissions.
- **Kode:**
  ```json
  {
    "server_id": "...",
    "scopes": ["bot", "applications.commands"],
    "permissions": "274877906944",
    "channel_access": "all"
  }
  ```
- **Verificering:** Install creates `app_installs` row and server member for bot; event emitted; uninstall removes access but preserves historical messages.
- **Commit:** `feat(apps): install applications to servers with grants`

### Task 11: Wire Authorized Apps settings
- **Fil(er):**
  - Modify `client/src/lib/components/settings/AuthorizedAppsPanel.svelte`
  - Create `crates/opencorde-api/src/routes/oauth2/authorized_apps.rs`
- **Hvad:** Replace stub. Users can see OAuth grants and revoke apps. Server owners can see installed apps in server settings.
- **Kode:** Endpoints:
  ```text
  GET    /api/v1/users/@me/authorized-apps
  DELETE /api/v1/users/@me/authorized-apps/{application_id}
  GET    /api/v1/servers/{server_id}/apps
  DELETE /api/v1/servers/{server_id}/apps/{application_id}
  ```
- **Verificering:** Revoked app token can no longer access protected user endpoints.
- **Commit:** `feat(settings): manage authorized apps`

---

## Fase 4 — Bot tokens, gateway intents and event delivery

### Task 12: Add bot token table and hashing
- **Fil(er):**
  - Create migration `0XX_bot_tokens.sql`
  - Create `crates/opencorde-db/src/repos/bot_token_repo.rs`
  - Create `crates/opencorde-core/src/models/bot_token.rs`
- **Hvad:** Bot tokens must be hash-backed, rotatable and revocable. Include intent bitmask and privileged intent approval state.
- **Kode:**
  ```sql
  CREATE TABLE bot_tokens (
    id BIGINT PRIMARY KEY,
    application_id BIGINT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    bot_user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_prefix VARCHAR(16) NOT NULL,
    token_hash TEXT NOT NULL,
    intents BIGINT NOT NULL DEFAULT 0,
    label VARCHAR(80),
    last_used_at TIMESTAMPTZ,
    created_by BIGINT NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ
  );
  ```
- **Verificering:** Plain token never appears in DB or list response; revoked token fails middleware instantly.
- **Commit:** `feat(bots): add hashed bot token storage`

### Task 13: Add bot token management API and portal UI
- **Fil(er):**
  - Create `crates/opencorde-api/src/routes/applications/tokens.rs`
  - Create `client/src/lib/components/developers/BotTokenPanel.svelte`
- **Hvad:** Owner/developer can create token, rotate, revoke, set label, choose requested intents. Plain token shown once with copy warning.
- **Kode:**
  ```text
  POST   /api/v1/applications/{id}/bot/tokens
  GET    /api/v1/applications/{id}/bot/tokens
  POST   /api/v1/applications/{id}/bot/tokens/{token_id}/rotate
  DELETE /api/v1/applications/{id}/bot/tokens/{token_id}
  ```
- **Verificering:** Playwright confirms token modal says “copy now; cannot be shown again”. API tests confirm no token in GET list.
- **Commit:** `feat(portal): manage bot tokens`

### Task 14: Define OpenCorde gateway opcode envelope
- **Fil(er):**
  - Modify `crates/opencorde-core/src/gateway.rs`
  - Create `crates/opencorde-core/src/bot_gateway.rs`
  - Modify `docs/developer-platform/api-contract.md`
- **Hvad:** Keep user client gateway compatibility, but add bot protocol envelope: `op`, `d`, `s`, `t`. Support Hello, Identify, Heartbeat, HeartbeatAck, Dispatch, Reconnect, Resume, InvalidSession.
- **Kode:**
  ```rust
  #[derive(Debug, Serialize, Deserialize)]
  pub struct GatewayPayload<T = serde_json::Value> {
      pub op: u8,
      pub d: Option<T>,
      pub s: Option<i64>,
      pub t: Option<String>,
  }
  ```
- **Verificering:** Unit tests serialize/deserialize opcodes and existing user events still compile.
- **Commit:** `feat(gateway): define bot opcode envelope`

### Task 15: Implement gateway intents model
- **Fil(er):**
  - Create `crates/opencorde-core/src/gateway_intents.rs`
  - Modify `crates/opencorde-core/src/permissions.rs` only if new `MANAGE_APPS` is added
  - Modify portal token UI
- **Hvad:** Add Discord-compatible intent buckets. Minimum: GUILDS, GUILD_MEMBERS privileged, GUILD_MODERATION, GUILD_MESSAGES, GUILD_MESSAGE_REACTIONS, DIRECT_MESSAGES, MESSAGE_CONTENT privileged, GUILD_VOICE_STATES, GUILD_PRESENCES privileged, INTEGRATIONS.
- **Kode:**
  ```rust
  bitflags::bitflags! {
      pub struct GatewayIntents: i64 {
          const GUILDS = 1 << 0;
          const GUILD_MEMBERS = 1 << 1;
          const GUILD_MODERATION = 1 << 2;
          const GUILD_MESSAGES = 1 << 9;
          const GUILD_MESSAGE_REACTIONS = 1 << 10;
          const GUILD_VOICE_STATES = 1 << 7;
          const GUILD_PRESENCES = 1 << 8;
          const DIRECT_MESSAGES = 1 << 12;
          const MESSAGE_CONTENT = 1 << 15;
          const INTEGRATIONS = 1 << 20;
      }
  }
  ```
- **Verificering:** Tests map event types to required intents; privileged intents cannot be enabled unless approved.
- **Commit:** `feat(gateway): add bot gateway intents`

### Task 16: Add bot gateway authentication and sessions
- **Fil(er):**
  - Modify `crates/opencorde-api/src/ws/handler.rs`
  - Create `crates/opencorde-api/src/ws/bot.rs`
  - Create `crates/opencorde-api/src/ws/session.rs`
  - Use Redis for session state where possible
- **Hvad:** Bot connects, receives Hello, sends Identify with token/intents, gets Ready with bot/app/install data. Store session_id, seq, intents, installed servers.
- **Kode:** Identify payload:
  ```json
  {
    "op": 2,
    "d": {
      "token": "ocb_...",
      "intents": 513,
      "properties": { "os": "linux", "library": "emma-opencorde" }
    }
  }
  ```
- **Verificering:** Integration test connects WS as bot, receives Hello -> sends Identify -> receives Ready -> heartbeats ack.
- **Commit:** `feat(gateway): authenticate bot websocket sessions`

### Task 17: Implement event filtering and resume
- **Fil(er):**
  - Create `crates/opencorde-api/src/ws/bot_events.rs`
  - Create `crates/opencorde-api/src/ws/event_buffer.rs`
- **Hvad:** Only dispatch events for servers where app is installed, channels bot can view, and intents requested/approved. Buffer last N events per session/app for resume.
- **Kode:** Filter rule:
  ```rust
  if !bot_install.allows_server(server_id) { return None; }
  if !intent_map.event_allowed(event_name, session.intents) { return None; }
  if event_contains_message_content && !session.intents.contains(MESSAGE_CONTENT) { redact_content(); }
  ```
- **Verificering:** Bot with GUILD_MESSAGES but no MESSAGE_CONTENT receives `MESSAGE_CREATE` with content redacted; bot with MESSAGE_CONTENT receives content.
- **Commit:** `feat(gateway): filter bot events by install and intents`

---

## Fase 5 — Application commands and interaction lifecycle

### Task 18: Replace shallow slash command schema with application commands
- **Fil(er):**
  - Create migration `0XX_application_commands.sql`
  - Modify/replace `crates/opencorde-db/migrations/024_slash_commands.sql` only through new migration, not edit old applied migration
  - Create `crates/opencorde-core/src/models/application_command.rs`
- **Hvad:** New command model is app-owned, supports server/global scope, type CHAT_INPUT/USER/MESSAGE, options, choices, autocomplete, default permissions and version.
- **Kode:**
  ```sql
  CREATE TABLE application_commands (
    id BIGINT PRIMARY KEY,
    application_id BIGINT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    server_id BIGINT REFERENCES servers(id) ON DELETE CASCADE,
    name VARCHAR(32) NOT NULL,
    description VARCHAR(100) NOT NULL DEFAULT '',
    command_type SMALLINT NOT NULL,
    options JSONB NOT NULL DEFAULT '[]',
    default_member_permissions BIGINT,
    dm_permission BOOLEAN NOT NULL DEFAULT FALSE,
    version BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );
  ```
- **Verificering:** Existing simple slash commands can still list during transition or are migrated into app commands with a legacy internal app.
- **Commit:** `feat(commands): add application command schema`

### Task 19: Add application command registry API
- **Fil(er):**
  - Create `crates/opencorde-api/src/routes/application_commands/mod.rs`
  - Create `create.rs`, `bulk.rs`, `list.rs`, `delete.rs`, `permissions.rs`
- **Hvad:** Bot/application token can register commands. Developer portal can manage them. Support global and server-specific commands.
- **Kode:** Routes:
  ```text
  GET/PUT/POST /api/v1/applications/{application_id}/commands
  GET/PUT/POST /api/v1/applications/{application_id}/guilds/{server_id}/commands
  DELETE       /api/v1/applications/{application_id}/commands/{command_id}
  ```
- **Verificering:** Bot token with `applications.commands` can register `/ping`; user token cannot register app command unless app owner.
- **Commit:** `feat(commands): add application command registry API`

### Task 20: Add interaction table and token lifecycle
- **Fil(er):**
  - Create migration `0XX_interactions.sql`
  - Create `crates/opencorde-core/src/models/interaction.rs`
  - Create `crates/opencorde-db/src/repos/interaction_repo.rs`
- **Hvad:** Store command/component/modal invocations with one-use interaction token, visibility, response state, expiration and followup eligibility.
- **Kode:**
  ```sql
  CREATE TABLE interactions (
    id BIGINT PRIMARY KEY,
    application_id BIGINT NOT NULL REFERENCES applications(id),
    token_hash TEXT NOT NULL,
    interaction_type SMALLINT NOT NULL,
    command_id BIGINT,
    server_id BIGINT,
    channel_id BIGINT,
    user_id BIGINT NOT NULL REFERENCES users(id),
    message_id BIGINT,
    data JSONB NOT NULL DEFAULT '{}',
    response_state VARCHAR(24) NOT NULL DEFAULT 'pending',
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    responded_at TIMESTAMPTZ
  );
  ```
- **Verificering:** Interaction token expires; second initial response after responded state returns 400.
- **Commit:** `feat(interactions): add interaction persistence`

### Task 21: Implement interaction callback API
- **Fil(er):**
  - Create `crates/opencorde-api/src/routes/interactions/callback.rs`
  - Create `crates/opencorde-api/src/routes/interactions/followups.rs`
- **Hvad:** Support callback types: Pong, ChannelMessageWithSource, DeferredChannelMessageWithSource, DeferredUpdateMessage, UpdateMessage, AutocompleteResult, Modal. Support ephemeral flag and followup messages.
- **Kode:** Routes:
  ```text
  POST  /api/v1/interactions/{interaction_id}/{token}/callback
  PATCH /api/v1/webhooks/{application_id}/{interaction_token}/messages/@original
  POST  /api/v1/webhooks/{application_id}/{interaction_token}
  ```
- **Verificering:** `/ping` can respond ephemeral; `/slow` can defer and then PATCH original response public.
- **Commit:** `feat(interactions): implement callback and followups`

### Task 22: Frontend slash autocomplete and command execution
- **Fil(er):**
  - Modify `client/src/lib/components/chat/CommandAutocomplete.svelte`
  - Modify `client/src/lib/components/chat/MessageInput.svelte`
  - Modify `client/src/lib/stores/slashCommands.svelte.ts`
- **Hvad:** Replace simple command list with option-aware autocomplete: choices, subcommands, focused option, validation. Hide commands the user cannot execute.
- **Kode:** UI state shape:
  ```ts
  type CommandDraft = {
    commandId: string;
    optionValues: Record<string, string | number | boolean>;
    focusedOption?: string;
  };
  ```
- **Verificering:** Playwright: `/` opens command list, selecting `/emma ping` shows option UI, Enter creates interaction and displays ephemeral response only to caller.
- **Commit:** `feat(commands): add option-aware command UI`

### Task 23: Add command permissions
- **Fil(er):**
  - Create migration `0XX_command_permissions.sql`
  - Create `crates/opencorde-api/src/routes/application_commands/permissions.rs`
  - Create `client/src/lib/components/settings/CommandPermissionsPanel.svelte`
- **Hvad:** Per-role, per-user, per-channel command access. Evaluate at autocomplete and execution time.
- **Kode:**
  ```sql
  CREATE TABLE application_command_permissions (
    command_id BIGINT NOT NULL REFERENCES application_commands(id) ON DELETE CASCADE,
    server_id BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    target_type VARCHAR(16) NOT NULL,
    target_id BIGINT NOT NULL,
    permission BOOLEAN NOT NULL,
    PRIMARY KEY (command_id, server_id, target_type, target_id)
  );
  ```
- **Verificering:** Denied command hidden in autocomplete and POST execution returns 403 if manually attempted.
- **Commit:** `feat(commands): add command permission overrides`

---

## Fase 6 — Rich embeds, components and webhook parity

### Task 24: Add message embed and component model
- **Fil(er):**
  - Create migration `0XX_message_embeds_components.sql`
  - Modify `crates/opencorde-core/src/models/message.rs`
  - Modify `crates/opencorde-db/src/repos/message_repo.rs`
- **Hvad:** Add full Discord-like embed fields: title, type, description, url, timestamp, color, footer, image, thumbnail, video, provider, author, fields. Add components JSON for buttons/selects.
- **Kode:** Prefer separate table for query/audit clarity:
  ```sql
  CREATE TABLE message_embeds (
    id BIGINT PRIMARY KEY,
    message_id BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    position SMALLINT NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );
  CREATE TABLE message_components (
    id BIGINT PRIMARY KEY,
    message_id BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    payload JSONB NOT NULL
  );
  ```
- **Verificering:** Message fetch returns embeds/components; legacy messages still work with empty arrays.
- **Commit:** `feat(messages): add rich embeds and components`

### Task 25: Add EmbedRenderer frontend component
- **Fil(er):**
  - Create `client/src/lib/components/chat/EmbedRenderer.svelte`
  - Create `client/src/lib/components/chat/MessageComponents.svelte`
  - Modify `client/src/lib/components/chat/MessageList.svelte`
- **Hvad:** Render all embed fields with Discord-like layout: color strip, author, title link, description markdown, fields grid, thumbnail, image, footer, timestamp. Render buttons/selects and emit component interactions.
- **Kode:** Component props:
  ```ts
  export interface MessageEmbed {
    title?: string; description?: string; url?: string; timestamp?: string;
    color?: number; footer?: { text: string; icon_url?: string };
    image?: { url: string }; thumbnail?: { url: string };
    author?: { name: string; url?: string; icon_url?: string };
    fields?: { name: string; value: string; inline?: boolean }[];
  }
  ```
- **Verificering:** Playwright screenshot for embed with title, fields, thumbnail, image and footer.
- **Commit:** `feat(chat): render rich embeds and components`

### Task 26: Upgrade webhooks to Discord-like execution
- **Fil(er):**
  - Modify `crates/opencorde-api/src/routes/webhooks.rs` or split into `routes/webhooks/`
  - Modify `crates/opencorde-db/migrations/018_webhooks.sql` via new migration only
  - Modify `crates/opencorde-db/src/repos/webhook_repo.rs`
- **Hvad:** Add webhook id + token route, token hash, avatar override, username override, embeds, attachments, allowed_mentions, wait, thread target, edit/delete original webhook messages.
- **Kode:** Routes:
  ```text
  POST  /api/v1/webhooks/{webhook_id}/{token}
  PATCH /api/v1/webhooks/{webhook_id}/{token}/messages/{message_id}
  DELETE /api/v1/webhooks/{webhook_id}/{token}/messages/{message_id}
  GET   /api/v1/webhooks/{webhook_id}/messages
  ```
- **Verificering:** `curl` can send embed payload with custom username/avatar; message author displays webhook identity, not creator user.
- **Commit:** `feat(webhooks): add Discord-style webhook execution`

### Task 27: Add webhook history and retry log
- **Fil(er):**
  - Create migration `0XX_webhook_deliveries.sql`
  - Create `crates/opencorde-db/src/repos/webhook_delivery_repo.rs`
  - Create background worker module if outbound webhooks are introduced
- **Hvad:** Store execution attempts, errors, response status, retry_count. For incoming webhooks, log every execution. For outgoing webhooks later, retry failed deliveries.
- **Kode:**
  ```sql
  CREATE TABLE webhook_deliveries (
    id BIGINT PRIMARY KEY,
    webhook_id BIGINT NOT NULL REFERENCES webhooks(id) ON DELETE CASCADE,
    message_id BIGINT,
    status VARCHAR(24) NOT NULL,
    request_payload JSONB,
    error TEXT,
    retry_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );
  ```
- **Verificering:** Webhook manager displays recent successful and failed deliveries.
- **Commit:** `feat(webhooks): record delivery history`

---

## Fase 7 — Audit log, integration logs and governance

### Task 28: Add typed audit action catalog
- **Fil(er):**
  - Create `crates/opencorde-core/src/audit_actions.rs`
  - Modify `crates/opencorde-api/src/routes/moderation/audit_mod.rs`
  - Modify `docs/developer-platform/api-contract.md`
- **Hvad:** Replace ad-hoc strings with typed action enum covering Discord-like actions: server/channel/role/member/message/webhook/app/command/token/automod changes.
- **Kode:**
  ```rust
  pub enum AuditAction {
      ChannelCreate, ChannelUpdate, ChannelDelete,
      MemberKick, MemberBanAdd, MemberBanRemove,
      RoleCreate, RoleUpdate, RoleDelete,
      WebhookCreate, WebhookUpdate, WebhookDelete,
      ApplicationInstall, ApplicationUninstall,
      BotTokenCreate, BotTokenRotate, BotTokenRevoke,
      CommandCreate, CommandUpdate, CommandDelete,
      AutomodRuleCreate, AutomodRuleTrigger,
  }
  ```
- **Verificering:** Existing audit route returns typed action plus legacy string where needed.
- **Commit:** `feat(audit): add typed audit action catalog`

### Task 29: Add integration logs separate from moderation audit
- **Fil(er):**
  - Create migration `0XX_integration_logs.sql`
  - Create `crates/opencorde-api/src/routes/integration_logs.rs`
  - Create `client/src/lib/components/settings/IntegrationLogPanel.svelte`
- **Hvad:** Audit log is “who changed server state”; integration log is “what apps/bots/webhooks did”. Keep both. Include app_id, bot_user_id, action, request_id, latency, status, metadata.
- **Kode:**
  ```sql
  CREATE TABLE integration_logs (
    id BIGINT PRIMARY KEY,
    server_id BIGINT REFERENCES servers(id) ON DELETE CASCADE,
    application_id BIGINT REFERENCES applications(id) ON DELETE SET NULL,
    actor_bot_user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    action_type VARCHAR(64) NOT NULL,
    status VARCHAR(24) NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );
  ```
- **Verificering:** Every command execution, webhook execution and token rotation creates an integration log row.
- **Commit:** `feat(integrations): add app and bot activity logs`

### Task 30: Expand AutoMod beyond keyword contains
- **Fil(er):**
  - Modify `crates/opencorde-api/src/automod.rs`
  - Modify `crates/opencorde-db/migrations/023_automod.sql` via new migration
  - Modify `client/src/lib/components/modals/AutomodManager.svelte`
- **Hvad:** Add mention spam threshold, repeated message spam, link/domain filters, exempt roles/channels, actions: block, flag, timeout, alert channel. Emit audit/integration events.
- **Kode:** Rule config JSON:
  ```json
  {
    "trigger_type": "mention_spam",
    "threshold": 5,
    "window_seconds": 10,
    "actions": [{ "type": "block" }, { "type": "alert", "channel_id": "..." }]
  }
  ```
- **Verificering:** API smoke: mention spam blocked; normal message allowed; action appears in audit log.
- **Commit:** `feat(automod): add spam and mention filters`

### Task 31: Add app trust and discovery metadata
- **Fil(er):**
  - Create migration `0XX_app_directory.sql`
  - Create `crates/opencorde-api/src/routes/app_directory.rs`
  - Create `client/src/routes/apps/+page.svelte`
- **Hvad:** App directory with public apps, tags, install counts, reviews, verification flag, privacy policy URL, terms URL. Start simple; this is platform growth/distribution.
- **Kode:**
  ```sql
  CREATE TABLE app_directory_entries (
    application_id BIGINT PRIMARY KEY REFERENCES applications(id) ON DELETE CASCADE,
    tags TEXT[] NOT NULL DEFAULT '{}',
    featured BOOLEAN NOT NULL DEFAULT FALSE,
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    install_count BIGINT NOT NULL DEFAULT 0,
    trust_score INT NOT NULL DEFAULT 0
  );
  ```
- **Verificering:** Public app can be listed, searched, opened and installed.
- **Commit:** `feat(apps): add app directory metadata`

---

## Fase 8 — Emma dual-platform bot

### Task 32: Split Emma into core logic plus platform adapters
- **Fil(er):**
  - In Emma repo/container source, create `emma_core/` and adapters after explicit permission to modify Thor files
  - For OpenCorde repo, create docs only first: `docs/developer-platform/emma-dual-platform.md`
- **Hvad:** Emma must not be “Red pretending to be OpenCorde”. Target architecture:
  - `EmmaCore`: commands, AI calls, knowledge/profile/music decisions.
  - `DiscordAdapter`: Red/discord.py events -> core -> Discord responses.
  - `OpenCordeAdapter`: OpenCorde gateway/interactions -> core -> OpenCorde API responses.
- **Kode:** Adapter interface:
  ```python
  class EmmaPlatformAdapter(Protocol):
      async def send_message(self, channel_id: str, content: str, embeds: list[dict] = ...) -> str: ...
      async def reply_interaction(self, interaction_id: str, token: str, response: dict) -> None: ...
      async def register_commands(self, commands: list[dict]) -> None: ...
  ```
- **Verificering:** Design doc maps every current Emma feature to Discord/OpenCorde support: chat, slash commands, welcome, profiles, KB, music/Lavalink.
- **Commit:** `docs(emma): define dual-platform adapter architecture`

### Task 33: Build OpenCorde Python client library for Emma
- **Fil(er):**
  - Create `scripts/emma_opencorde_client.py` or separate Emma-side package after permission
  - Create `docs/testing/emma-opencorde.env.example` with no secrets
- **Hvad:** Minimal typed client: bot token auth, gateway connect, heartbeat, identify, register commands, respond interactions, send messages, add reactions, create threads/forum posts, RSVP.
- **Kode:**
  ```python
  class OpenCordeBotClient:
      def __init__(self, base_url: str, gateway_url: str, token: str): ...
      async def connect_gateway(self, intents: int): ...
      async def register_command(self, app_id: str, command: dict): ...
      async def respond_interaction(self, interaction_id: str, token: str, data: dict): ...
  ```
- **Verificering:** Unit test with mocked WS proves Identify/Heartbeat/Dispatch flow.
- **Commit:** `test(emma): add OpenCorde bot client harness`

### Task 34: Register Emma as real OpenCorde application
- **Fil(er):**
  - Create `scripts/register_emma_app.py`
  - Create `docs/testing/emma-bot.md` update for real app path
- **Hvad:** Script creates/updates Emma application, bot user, token and commands using OpenCorde APIs. It must read credentials from `~/.hermes/opencorde/emma-bot.env` or operator-provided `.env`, never repo.
- **Kode:** Script flow:
  ```text
  load env -> login operator/test owner -> create/find app "Emma" -> create/find bot -> create token if absent -> register /emma ping /emma status /emma test -> print redacted report
  ```
- **Verificering:** Sanitized report proves app_id, bot_user_id, commands registered; token redacted.
- **Commit:** `feat(emma): add real OpenCorde app registration script`

### Task 35: Emma interaction acceptance
- **Fil(er):**
  - Create `scripts/emma_interactions_qa.py`
  - Create `reports/raw/emma-opencorde-interactions-proof.json`
- **Hvad:** Live QA: user invokes `/emma ping` -> ephemeral response; `/emma status` -> public rich embed; `/emma test` -> deferred then public final; Emma receives message event through gateway.
- **Kode:** Report schema:
  ```json
  {
    "target": "https://opencorde.com",
    "scenarios": [
      { "name": "ephemeral_ping", "expected": "only caller sees Pong", "passed": true },
      { "name": "public_status_embed", "expected": "channel sees embed", "passed": true },
      { "name": "gateway_message_event", "expected": "Emma receives MESSAGE_CREATE", "passed": true }
    ],
    "secrets_exposed": false
  }
  ```
- **Verificering:** Playwright two-user proof confirms ephemeral message is invisible to second user.
- **Commit:** `test(emma): prove OpenCorde interaction flows`

### Task 36: Emma dual-platform feature parity matrix
- **Fil(er):**
  - Create `docs/developer-platform/emma-feature-matrix.md`
- **Hvad:** Map current Emma features:
  - Chat: Discord message event + OpenCorde MESSAGE_CREATE.
  - Slash commands: Discord app commands + OpenCorde application commands.
  - Welcome: Discord member join + OpenCorde MemberJoin.
  - Profiles: existing storage + OpenCorde users/members.
  - Knowledge base: same AI Gateway/Kurt access, no OpenAI direct calls.
  - Music: Discord Lavalink remains Discord-only until OpenCorde voice bot API supports playback.
- **Kode:** Include status columns: Discord live, OpenCorde MVP, OpenCorde blocked-by.
- **Verificering:** Matrix has no “unknown” cells for current Emma features.
- **Commit:** `docs(emma): add dual-platform feature matrix`

---

## Fase 9 — QA, live proof and kanban alignment

### Task 37: Add platform API smoke suite
- **Fil(er):**
  - Create `scripts/platform_api_qa.py`
  - Create `reports/raw/platform-api-proof.json`
- **Hvad:** End-to-end API smoke for app creation, bot creation, token creation, OAuth grant, install, command register, webhook execute, audit/integration logs.
- **Kode:** Scenario order:
  ```text
  register owner -> create server/channel -> create app -> create bot -> create token -> install app -> register command -> execute interaction -> send webhook embed -> verify logs -> revoke token -> assert token denied
  ```
- **Verificering:** Report has `checks_total`, `checks_passed`, `checks_failed=0`, and redacts tokens.
- **Commit:** `test(platform): add developer API smoke proof`

### Task 38: Add platform Playwright suite
- **Fil(er):**
  - Create `scripts/platform_ui_qa.py`
  - Create screenshots under `reports/parity-screenshots/developer-platform/`
- **Hvad:** UI proof: Developer Portal app creation, token one-time modal, install preview, server integrations panel, command autocomplete, ephemeral response, rich embed render, authorized app revoke.
- **Kode:** Use stable selectors with `data-testid` added to components as needed.
- **Verificering:** Screenshots + JSON report prove desktop and at least mobile/tablet layout for portal pages.
- **Commit:** `test(platform): add developer portal UI proof`

### Task 39: Update parity report and kanban epics
- **Fil(er):**
  - Modify `reports/discord-parity.json`
  - Use kanban board `opencorde` after implementation, not in this planning pass
- **Hvad:** Do not mark Apps/Bots/Webhooks “proven” until live evidence exists. Split status by sub-area if needed: app control plane, gateway, interactions, webhooks, Emma.
- **Kode:** Add sub-area status fields:
  ```json
  "platform_breakdown": {
    "developer_portal": "partial",
    "bot_gateway": "missing",
    "interactions": "partial",
    "webhooks": "partial",
    "emma_open_corde_bot": "missing"
  }
  ```
- **Verificering:** `python3 -m json.tool reports/discord-parity.json >/dev/null`.
- **Commit:** `docs(parity): track developer platform subareas`

### Task 40: Final live acceptance gate
- **Fil(er):**
  - `reports/raw/platform-api-proof.json`
  - `reports/raw/platform-ui-proof.json`
  - `reports/raw/emma-opencorde-interactions-proof.json`
  - `reports/discord-parity.json`
- **Hvad:** The milestone is done only when all acceptance scenarios pass against `https://opencorde.com`:
  1. Developer creates app and bot in portal.
  2. Bot token shown once, can be rotated/revoked.
  3. App installed to server with scopes/permissions.
  4. Bot connects to gateway with intents and receives filtered events.
  5. Slash command returns ephemeral response visible only to caller.
  6. Slash command returns public rich embed visible to channel.
  7. Webhook sends message with embed/avatar/username override.
  8. Audit/integration logs show app install, command run, webhook execution, token rotation.
  9. Emma uses real OpenCorde app/bot path, no DB shortcut.
- **Kode:** No code; this is an evidence gate.
- **Verificering:** All reports show zero failures; live deployed site verified before any success claim.
- **Commit:** `test(platform): prove OpenCorde developer platform parity`

---

## Implementation order for subagents

**Critical path:**
1. Task 1 -> Task 4 -> Task 5 -> Task 8 -> Task 10 -> Task 12 -> Task 16 -> Task 18 -> Task 20 -> Task 21 -> Task 35 -> Task 40.

**Can run parallel after Task 1:**
- Developer Portal UI (Tasks 6-7) after API skeleton.
- OAuth client/authorized apps (Tasks 8-11).
- Webhook/embeds (Tasks 24-27) after app/bot message author semantics are decided.
- Governance (Tasks 28-31) can start once typed action catalog exists.

**Do not start until dependencies are real:**
- Emma acceptance (Tasks 34-35) waits for bot token, gateway, commands, interactions.
- Command permissions (Task 23) waits for application commands and interaction execution.
- Rich component interactions wait for interaction callback API.

---

## Acceptance definition: “Discord-level depth”

OpenCorde is deep enough when a third-party developer can do this without internal help:

1. Open `/developers`, create app “ExampleBot”.
2. Add bot user and generate token.
3. Choose intents; privileged intents require owner/admin approval.
4. Install bot to a server through OpenCorde OAuth/install flow.
5. Register `/hello` command with choices/autocomplete.
6. Connect to gateway and receive `MESSAGE_CREATE` only in allowed channels.
7. Respond to `/hello` with ephemeral message.
8. Send a public embed with fields/image/footer.
9. Add a button to the message and receive component interaction.
10. Rotate token and see old token fail.
11. Server owner sees audit/integration logs for install, command, webhook and token actions.
12. User sees the app in Authorized Apps and can revoke it.
13. Emma runs through the exact same APIs.

If any of these require a script that bypasses public API, DB writes, or privileged internal route, the platform is not done.

---

## Notes for Hermes execution

- Use TDD for all backend modules: model tests -> repo tests -> route tests -> live smoke.
- Keep every Rust/Svelte file under 300 lines; split early.
- Never edit historical applied migrations; add new migrations.
- Regenerate `reports/raw/route-inventory.json` after route changes.
- Always verify live site before “proven” status.
- Keep tokens out of stdout, screenshots, reports and kanban comments.
- Update `ReadMeFirst.md`, `project-map.yaml`, `INDEX.md` only when structure actually changes.
- Use Conventional Commits.
