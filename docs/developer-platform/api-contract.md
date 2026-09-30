# OpenCorde Developer API Contract

> Version 1.0 — 2026-06-09  
> Target: Discord-compatible semantics, OpenCorde-specific paths under `/api/v1`

## Compatibility Rule

OpenCorde does not need byte-for-byte Discord API compatibility, but every Discord platform concept must map to a stable OpenCorde object, permission, event, and audit action.

## Bot Token Rule

Plain token is returned exactly once at creation. Server stores Argon2id hash plus non-secret prefix. Subsequent API responses show only the prefix (e.g., `ocb_AbCd...`). Rotation generates a new token; old token is invalidated immediately.

## Object Schemas

### Application
```
id             BIGINT (snowflake)   — unique identifier
name           VARCHAR(32)          — unique per owner
description    TEXT?                 — optional description
icon_url       TEXT?                 — optional icon
owner_user_id  BIGINT                — creator/owner
flags          BIGINT                — bitfield (public, verified, etc.)
is_public      BOOLEAN               — listed in app directory
redirect_uris  JSONB (string[])      — allowed OAuth2 redirect URIs
created_at     TIMESTAMPTZ
updated_at     TIMESTAMPTZ
```

### BotUser
```
id              BIGINT (snowflake)   — references users.id
application_id  BIGINT                — owning application
username        VARCHAR(32)           — display name
avatar_url      TEXT?                 — optional avatar
created_at      TIMESTAMPTZ
```
Note: Bot user is a real `users` row with `is_bot=true`, no password login, no refresh tokens.

### BotToken
```
id              BIGINT (snowflake)
application_id  BIGINT
bot_user_id     BIGINT
token_prefix    VARCHAR(16)           — e.g., "ocb_AbCd1234"
token_hash      TEXT                  — Argon2id hash (never returned)
intents         BIGINT                — requested gateway intents bitfield
label           VARCHAR(80)?          — human-readable label
last_used_at    TIMESTAMPTZ?
created_by      BIGINT
created_at      TIMESTAMPTZ
revoked_at      TIMESTAMPTZ?          — null = active
```

### OAuthScope
```
id          SMALLINT    — 1=identify, 2=guilds, 3=bot, 4=messages.read, 5=applications.commands, 6=webhook.incoming
name        VARCHAR(64) — unique scope name
description TEXT?
```

### AppInstall
```
id              BIGINT (snowflake)
application_id  BIGINT
server_id       BIGINT
installed_by    BIGINT                — user who authorized install
scopes          TEXT[]                — granted OAuth2 scopes
permissions     BIGINT?               — granted permission bitfield
created_at      TIMESTAMPTZ
```

### ApplicationCommand
```
id                          BIGINT (snowflake)
application_id              BIGINT
server_id                   BIGINT?       — null = global command
name                        VARCHAR(32)   — lowercase, no spaces
description                 VARCHAR(100)  — for CHAT_INPUT type
command_type                SMALLINT      — 1=CHAT_INPUT, 2=USER, 3=MESSAGE
options                     JSONB         — array of option objects
default_member_permissions  BIGINT?       — required permission bits
dm_permission               BOOLEAN       — allowed in DMs?
version                     BIGINT        — increment on update
created_at                  TIMESTAMPTZ
updated_at                  TIMESTAMPTZ
```

### Interaction
```
id               BIGINT (snowflake)
application_id   BIGINT
token_hash       TEXT                  — one-time interaction token hash
interaction_type SMALLINT              — 2=application_command, 3=message_component, 4=modal_submit, 5=autocomplete
command_id       BIGINT?               — if triggered by a command
server_id        BIGINT?
channel_id       BIGINT?
user_id          BIGINT                — invoking user
message_id       BIGINT?               — if component interaction
data             JSONB                 — command options, component values, etc.
response_state   VARCHAR(24)           — pending, responded, deferred
expires_at       TIMESTAMPTZ           — token expiration (15 min from creation)
created_at       TIMESTAMPTZ
responded_at     TIMESTAMPTZ?
```

### InteractionResponse
```
type    INTEGER   — 1=PONG, 4=CHANNEL_MESSAGE_WITH_SOURCE, 5=DEFERRED_CHANNEL_MESSAGE_WITH_SOURCE, 6=DEFERRED_UPDATE_MESSAGE, 7=UPDATE_MESSAGE, 8=AUTOCOMPLETE_RESULT, 9=MODAL
data    JSON?     — depends on type: message payload, autocomplete choices, modal definition
```

### MessageEmbed
```
title       VARCHAR(256)?
type        VARCHAR(32)?   — "rich", "image", "video", "article", "link"
description TEXT?           — markdown support
url         TEXT?
timestamp   TIMESTAMPTZ?
color       INTEGER?        — decimal color value
footer      { text: VARCHAR(2048), icon_url?: TEXT, proxy_icon_url?: TEXT }?
image       { url: TEXT, proxy_url?: TEXT, height?: INT, width?: INT }?
thumbnail   { url: TEXT, proxy_url?: TEXT, height?: INT, width?: INT }?
video       { url: TEXT, height?: INT, width?: INT }?
provider    { name: TEXT, url?: TEXT }?
author      { name: VARCHAR(256), url?: TEXT, icon_url?: TEXT, proxy_icon_url?: TEXT }?
fields      [{ name: VARCHAR(256), value: VARCHAR(1024), inline?: BOOLEAN }]?
```

### Webhook
```
id              BIGINT (snowflake)
server_id       BIGINT
channel_id      BIGINT
name            VARCHAR(80)           — default webhook name
avatar_url      TEXT?                 — default avatar
token_hash      TEXT                  — execution token hash (plain token shown once)
created_by      BIGINT
application_id  BIGINT?               — if owned by an application
created_at      TIMESTAMPTZ
```

### AuditLogEntry
```
id          BIGINT (snowflake)
server_id   BIGINT?
action_type VARCHAR(64)   — typed action: ChannelCreate, MemberBanAdd, ApplicationInstall, BotTokenCreate, CommandCreate, etc.
user_id     BIGINT?       — acting user (null for system actions)
target_id   BIGINT?       — affected entity
changes     JSONB?        — before/after diff
reason      TEXT?         — optional reason
created_at  TIMESTAMPTZ
```

### IntegrationLogEntry
```
id               BIGINT (snowflake)
server_id        BIGINT?
application_id   BIGINT?
actor_bot_user_id BIGINT?       — bot that performed the action
action_type      VARCHAR(64)    — command_execution, webhook_execution, token_rotation, interaction_handled
status           VARCHAR(24)    — success, failed, rate_limited
metadata         JSONB          — request_id, latency_ms, error_details
created_at       TIMESTAMPTZ
```

---

## Gateway Protocol

### Opcode Envelope
```
{
  "op": u8,              // 0=Dispatch, 1=Heartbeat, 7=Reconnect, 9=InvalidSession, 10=Hello, 11=HeartbeatAck
  "d": object?,           // event data
  "s": i64?,              // sequence number (for resume)
  "t": string?            // event name (for Dispatch)
}
```

### Gateway Intents
```
GUILDS                  = 1 << 0   (1)
GUILD_MEMBERS           = 1 << 1   (2)  [PRIVILEGED]
GUILD_MODERATION        = 1 << 2   (4)
GUILD_MESSAGES          = 1 << 9   (512)
GUILD_MESSAGE_REACTIONS = 1 << 10  (1024)
GUILD_VOICE_STATES      = 1 << 7   (128)
GUILD_PRESENCES         = 1 << 8   (256)  [PRIVILEGED]
DIRECT_MESSAGES         = 1 << 12  (4096)
MESSAGE_CONTENT         = 1 << 15  (32768) [PRIVILEGED]
INTEGRATIONS            = 1 << 20  (1048576)
```

---

## API Routes

### Applications
```
POST   /api/v1/applications                          — Create application
GET    /api/v1/applications/@me                      — List my applications
GET    /api/v1/applications/public                   — List public applications (app directory)
GET    /api/v1/applications/{id}                     — Get application
PATCH  /api/v1/applications/{id}                     — Update application
DELETE /api/v1/applications/{id}                     — Delete application
POST   /api/v1/applications/{id}/bot                 — Create bot user
POST   /api/v1/applications/{id}/bot/tokens          — Create bot token
GET    /api/v1/applications/{id}/bot/tokens          — List bot tokens
POST   /api/v1/applications/{id}/bot/tokens/{tid}/rotate  — Rotate token
DELETE /api/v1/applications/{id}/bot/tokens/{tid}    — Revoke token
POST   /api/v1/applications/{id}/install             — Install app to server
GET    /api/v1/applications/{id}/commands             — List commands (global)
PUT    /api/v1/applications/{id}/commands             — Bulk overwrite commands
GET    /api/v1/applications/{id}/guilds/{sid}/commands — List server commands
PUT    /api/v1/applications/{id}/guilds/{sid}/commands — Bulk overwrite server commands
```

### OAuth2
```
GET    /api/v1/oauth2/authorize  — Authorization prompt
POST   /api/v1/oauth2/authorize  — User approval
POST   /api/v1/oauth2/token      — Exchange code for access token
POST   /api/v1/oauth2/revoke     — Revoke grant
GET    /api/v1/users/@me/authorized-apps           — List authorized apps
DELETE /api/v1/users/@me/authorized-apps/{app_id}  — Revoke app authorization
GET    /api/v1/servers/{id}/apps                    — List installed apps
DELETE /api/v1/servers/{id}/apps/{app_id}           — Uninstall app
```

### Interactions
```
POST   /api/v1/interactions/{id}/{token}/callback   — Initial response
PATCH  /api/v1/webhooks/{app_id}/{token}/messages/@original  — Edit original
POST   /api/v1/webhooks/{app_id}/{token}            — Send followup
```

### Webhooks
```
POST   /api/v1/webhooks/{id}/{token}                — Execute webhook
PATCH  /api/v1/webhooks/{id}/{token}/messages/{mid} — Edit webhook message
DELETE /api/v1/webhooks/{id}/{token}/messages/{mid} — Delete webhook message
GET    /api/v1/webhooks/{id}/messages               — Webhook message history
```
