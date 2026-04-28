# OpenCorde Discord Parity Master Plan

Status: Draft v1, 2026-04-28
Owner: OpenCorde maintainers + AI agents
Primary goal: Make OpenCorde feel and function like a self-hosted Discord alternative, while keeping OpenCorde legally and technically its own product.

## North Star

OpenCorde should support the same day-to-day workflows that make Discord useful:

- A user can join, discover, and manage communities.
- A server owner can configure roles, permissions, onboarding, moderation, channels, integrations, bots, webhooks, events, and safety.
- Members can chat, react, reply, thread, search, upload, voice chat, screen share, use forum channels, join stage events, message friends, and manage notifications.
- Developers can build bots/apps with slash commands, message/user context commands, webhooks, modals, buttons, and permission-scoped installs.
- The product feels like a persistent client, not a sequence of web pages.

Non-goals:

- Do not copy Discord branding, assets, names, or proprietary UI pixel-for-pixel.
- Do not implement Nitro monetization as-is. Implement equivalent extensibility and optional server support features only if they make sense for self-hosting.
- Do not chase every experimental Discord feature before core parity is reliable.

## Reference Baseline

Use Discord's public docs and support pages as the feature baseline:

- Discord permissions and channel overwrites: https://docs.discord.com/developers/topics/permissions
- Discord application commands, user commands, and message commands: https://docs.discord.com/developers/docs/interactions/slash-commands
- Discord interactions model: https://docs.discord.com/developers/platform/interactions
- Discord forum channels: https://support.discord.com/hc/en-us/articles/6208479917079-Forum-Channels-FAQ
- Discord stage channels: https://support.discord.com/hc/en-us/articles/1500005513722-Stage-Channels-FAQ
- Discord server guide/onboarding concepts: https://support.discord.com/hc/en-us/articles/13497665141655-Server-Guide-Beta

## Current Reality: Shallow Skeletons, Not Parity

OpenCorde has many Discord-shaped names, tables, endpoints, stores, and UI panels. That is useful raw material, but it must not be treated as Discord parity.

Surface-level skeletons observed:

- Auth: register, login, refresh tokens, email verification, password reset, Steam login, 2FA/TOTP.
- Server model: servers, channels, members, invites, roles, role assignment, channel permission overrides.
- Messaging: text messages, edits/deletes, replies, reactions, attachments, pins, read state, typing, search, unfurl.
- Realtime: WebSocket gateway events, presence scaffolding, message/channel/role/member/server events.
- Voice/video: LiveKit integration, voice state, device settings, video grid, recording, stage channels, soundboard.
- Community tools: forum channels, scheduled events, onboarding, server guide surface, audit log, moderation routes.
- Integrations: slash command model, webhooks, Discord bridge, bridge ghost users.
- Admin: stats, users, servers, rate limits, storage.
- Desktop/mobile foundation: Tauri, PWA-ish static web build, mobile notes/stubs.

Hard truth:

Most of the above should be assumed shallow or partial until proven by a live workflow. A schema, route, store, or component is not a finished feature. It only becomes finished when a user can discover it, use it without special knowledge, see correct realtime behavior, hit correct permission boundaries, and pass automated browser/API/Emma checks.

Default project posture:

- User is right until the product proves otherwise.
- Do not call anything "done" because a route exists.
- Do not count a Discord-looking UI surface as parity until the whole user journey works.
- Demote every inherited claim to "Shallow/needs proof" unless there is current evidence.

## Definition of Done

A feature is only "Discord-parity done" when all of these are true:

- UI entry point exists in the same mental location a Discord user expects.
- Backend endpoint and database model exist.
- Permission checks match the user's effective server/channel permissions.
- WebSocket/realtime updates keep other clients in sync.
- Mobile and desktop layouts are usable without hidden controls.
- Playwright covers happy path and at least one denied/empty/error path.
- Emma Bot can exercise the feature in a live server.
- Documentation and screenshots are updated.

No checkbox in this plan gets marked `[x]` until the proof is attached in the related issue, PR, or milestone report.

## Project Structure

Track implementation in four layers:

1. Parity map: what Discord has, what OpenCorde has, exact gap.
2. Vertical milestones: ship coherent user workflows, not isolated endpoints.
3. Quality gates: browser tests, API tests, permission tests, migration checks.
4. Emma Bot dogfooding: every shipped feature is used by an automated in-app actor.

## Roles

- Product owner: prioritizes parity gaps and decides when "Discord-like enough" is reached.
- Backend agent: Rust/API/database/migrations/realtime events.
- Frontend agent: Svelte UI, navigation, responsive shell, Playwright.
- QA agent: browser tests, screenshots, regression matrix, bug filing.
- Emma Bot: in-channel live user simulator, feature exerciser, regression reporter.

## Emma Bot Usage

Emma should be treated as a live product tester inside OpenCorde.

Required Emma capabilities:

- Join a test server.
- Send text messages.
- Reply in threads.
- React to messages.
- Upload attachments.
- Create forum posts.
- RSVP to events.
- Join/leave voice or simulated voice state where possible.
- Execute slash commands.
- Trigger moderation test cases with safe phrases.
- Report results back into a QA channel.

Emma task protocol:

- Every milestone gets an `Emma Script`.
- Emma Script is written in plain language plus expected UI/API outcome.
- Emma posts one summary message per run:
  - feature tested
  - pass/fail
  - observed UI/API result
  - screenshot or link if available
  - regression ticket if failed

## Parity Matrix

Legend:

- Proven: complete end-to-end, dogfooded, tested, and screenshot/API evidence attached.
- Shallow/needs proof: backend, schema, store, or UI exists, but the full workflow has not been proven.
- Partial: meaningful workflow exists, but it is incomplete or missing polish, permissions, realtime, mobile, or tests.
- Missing: no meaningful implementation.
- Unknown: needs audit.

### Account and Identity

- [ ] Shallow/needs proof: Email/password login.
- [ ] Shallow/needs proof: Password reset.
- [ ] Shallow/needs proof: Email verification.
- [ ] Shallow/needs proof: 2FA/TOTP.
- [ ] Shallow/needs proof: Steam OAuth.
- [ ] Sessions/devices page.
- [ ] Authorized apps page.
- [ ] Account switcher.
- [ ] QR login / device handoff.
- [ ] User badges and profile decorations.
- [ ] Pronouns / richer profile metadata.
- [ ] Status picker parity: Online, Idle, DND, Invisible, custom expiration.
- [ ] Activity status: playing/listening/streaming/custom app presence.

### Friends and DMs

- [ ] Shallow/needs proof: Friends store and page.
- [ ] Shallow/needs proof: DM channel routes.
- [ ] Group DMs.
- [ ] DM call controls.
- [ ] Message requests / spam inbox.
- [ ] Friend nicknames.
- [ ] Mutual servers/mutual friends view.
- [ ] Blocked users management with clear privacy behavior.
- [ ] Per-DM notification overrides.

### Server Discovery and Joining

- [ ] Shallow/needs proof: Server create/join route.
- [ ] Shallow/needs proof: Invite endpoint.
- [ ] Shallow/needs proof: Discovery route.
- [ ] Public server directory with categories, search, tags, language, member count, activity score.
- [ ] Invite preview page before joining.
- [ ] Invite expiration, max uses, temporary membership.
- [ ] Vanity invite codes.
- [ ] Join request workflow for gated servers.
- [ ] Server boost/support equivalent for self-hosted instances.

### Server Home and Guide

- [ ] Shallow/needs proof: Server home overview.
- [ ] Shallow/needs proof: Onboarding endpoint and modal.
- [ ] Shallow/needs proof: Server guide-style welcome surface.
- [ ] Default channel selection like Discord onboarding.
- [ ] Onboarding questions with role/channel assignment.
- [ ] Resource pages / server guide cards.
- [ ] New member action checklist.
- [ ] Rules screening before membership.
- [ ] Membership screening audit trail.

### Channels

- [ ] Shallow/needs proof: Text channels.
- [ ] Shallow/needs proof: Voice channels.
- [ ] Shallow/needs proof: Stage channels.
- [ ] Shallow/needs proof: Forum channels.
- [ ] Shallow/needs proof: Announcement channel type.
- [ ] Category drag/drop sorting.
- [ ] Channel drag/drop sorting.
- [ ] Clone channel.
- [ ] Channel templates.
- [ ] Slowmode UI per channel.
- [ ] NSFW channel gate.
- [ ] Private channel setup wizard.
- [ ] Browse channels surface for large servers.
- [ ] Media/gallery channel equivalent.
- [ ] Rules channel/system messages configuration.

### Messaging

- [ ] Shallow/needs proof: Send/edit/delete.
- [ ] Shallow/needs proof: Replies.
- [ ] Shallow/needs proof: Reactions.
- [ ] Shallow/needs proof: Attachments.
- [ ] Shallow/needs proof: Pins.
- [ ] Shallow/needs proof: Typing.
- [ ] Shallow/needs proof: Markdown rendering.
- [ ] Shallow/needs proof: Link unfurl.
- [ ] Message context actions parity: copy link, mark unread, report, pin/unpin, create thread, apps.
- [ ] Mentions: user, role, channel, everyone/here with permission gates.
- [ ] Jump-to-message deep links.
- [ ] Infinite scroll and cursor restoration.
- [ ] Message grouping by author/time.
- [ ] Rich embeds with provider cards.
- [ ] Poll messages.
- [ ] Voice messages.
- [ ] Sticker messages.
- [ ] GIF picker integration.
- [ ] Emoji autocomplete.
- [ ] File upload progress/cancel/retry.
- [ ] Spoiler attachments.
- [ ] Message forwarding/share to channel.
- [ ] Scheduled/send-later messages.
- [ ] Local drafts per channel.

### Threads and Forums

- [ ] Shallow/needs proof: Thread panel.
- [ ] Shallow/needs proof: Forum posts/replies.
- [ ] Public/private thread distinction.
- [ ] Auto-archive durations.
- [ ] Thread membership and notifications.
- [ ] Forum tags.
- [ ] Forum post sorting/filtering.
- [ ] Forum guidelines and default reactions.
- [ ] Mark answer/solution for forum posts.
- [ ] Cross-channel thread search.

### Search

- [ ] Shallow/needs proof: Message/channel/member quick switcher surface.
- [ ] Shallow/needs proof: API search path.
- [ ] Discord-like search syntax: from:, mentions:, has:, before:, after:, in:, pinned:, file type.
- [ ] Search result message preview with jump.
- [ ] Per-server index health UI.
- [ ] Search index creation/repair admin action.
- [ ] Federated search policy.

### Voice, Video, Screen Share

- [ ] Shallow/needs proof: LiveKit voice/video.
- [ ] Shallow/needs proof: Device selection.
- [ ] Shallow/needs proof: Video grid.
- [ ] Shallow/needs proof: Recordings.
- [ ] Shallow/needs proof: Soundboard.
- [ ] Shallow/needs proof: Stage panel.
- [ ] Screen share / Go Live UX.
- [ ] Stream preview tiles.
- [ ] Voice activity indicators in channel list and member list.
- [ ] Noise suppression toggle.
- [ ] Push-to-talk.
- [ ] Input sensitivity.
- [ ] Per-user volume.
- [ ] Mute/deafen/server mute/server deafen enforcement.
- [ ] Move users between voice channels.
- [ ] Voice region/quality controls.
- [ ] Voice invite/deep link.
- [ ] Activities in voice channels.
- [ ] Clips.

### Roles and Permissions

- [ ] Shallow/needs proof: Permission bit definitions aligned with Discord-style flags.
- [ ] Shallow/needs proof: Role APIs and stores.
- [ ] Shallow/needs proof: Channel overrides.
- [ ] Full role editor with create/edit/delete/reorder/color/icon/hoist/mentionable.
- [ ] Effective permissions inspector.
- [ ] Permission matrix UI for roles and channel overwrites.
- [ ] Member-specific channel overwrites.
- [ ] App/slash command permission overrides.
- [ ] Role hierarchy enforcement everywhere.
- [ ] "View as role/member" preview.
- [ ] Bulk role assignment.
- [ ] Role subscription/support equivalents, if desired.

### Moderation and Safety

- [ ] Shallow/needs proof: Audit log.
- [ ] Shallow/needs proof: Bans/timeouts/moderation routes.
- [ ] Shallow/needs proof: AutoMod scaffolding.
- [ ] Shallow/needs proof: Rate limits.
- [ ] Discord-like AutoMod rule builder UI with triggers/actions/exemptions.
- [ ] Warn/kick/ban/timeout UX from member context menu.
- [ ] Mod queue.
- [ ] Reports system.
- [ ] Safety setup checklist.
- [ ] Raid protection.
- [ ] Verification levels surfaced and enforced visibly.
- [ ] Content filter settings.
- [ ] Audit log filters/search/export.
- [ ] Permission-gated moderation dashboard.

### Events and Stages

- [ ] Shallow/needs proof: Scheduled events route.
- [ ] Shallow/needs proof: RSVP API.
- [ ] Shallow/needs proof: Stage channels.
- [ ] Event location tied to channel/stage/voice/external link.
- [ ] Event cover image.
- [ ] Event reminder notifications.
- [ ] Event start/end lifecycle.
- [ ] Speaker/moderator controls for stage events.
- [ ] Event recurrence.
- [ ] Calendar export.

### Apps, Bots, Webhooks, and Integrations

- [ ] Shallow/needs proof: Webhooks.
- [ ] Shallow/needs proof: Slash command model.
- [ ] Shallow/needs proof: Discord bridge.
- [ ] Full interactions model: buttons, selects, modals, ephemeral responses.
- [ ] User commands.
- [ ] Message commands.
- [ ] Command permissions UI.
- [ ] App install flow.
- [ ] Bot token management.
- [ ] OAuth scopes and grants.
- [ ] App directory.
- [ ] Webhook execution history and retry.
- [ ] Integration logs.
- [ ] Per-channel app access.

### Notifications

- [ ] Shallow/needs proof: Notification settings store.
- [ ] Shallow/needs proof: Push notification scaffolding.
- [ ] Server notification override UI.
- [ ] Channel notification override UI.
- [ ] Mention-only / nothing / all messages modes.
- [ ] Mute server/channel until time.
- [ ] Mobile push delivery.
- [ ] Desktop notifications from web and Tauri.
- [ ] Badge counts.
- [ ] Unread separators.
- [ ] Inbox / recent mentions.

### Admin, Instance, and Federation

- [ ] Shallow/needs proof: Instance admin dashboard.
- [ ] Shallow/needs proof: Federation peer registry.
- [ ] Shallow/needs proof: Mesh peer model.
- [ ] Federation UX: remote users, remote DMs, remote server trust, moderation boundaries.
- [ ] Instance setup wizard.
- [ ] Backup/restore.
- [ ] SMTP test button.
- [ ] Storage quota controls.
- [ ] Instance branding.
- [ ] Health dashboard.
- [ ] Background job dashboard.
- [ ] Version/update check.

### Desktop and Mobile

- [ ] Shallow/needs proof: Tauri desktop foundation.
- [ ] Shallow/needs proof: System tray / notifications claimed in tasks.
- [ ] Verify desktop packaging on Windows/macOS/Linux.
- [ ] Installer update flow.
- [ ] Mobile iOS/Android build completion.
- [ ] Native push.
- [ ] Mobile navigation parity.
- [ ] Offline cache and reconnect UX.
- [ ] Deep link handling tests.

## Milestone Roadmap

### Milestone -1: Brutal Audit and Claim Demotion

Goal: Remove false confidence before building more surface area.

TODO:

- [ ] Review every row in the parity matrix and keep it unchecked unless there is fresh proof.
- [ ] For each shallow feature, record whether the gap is UI, API, database, realtime, permissions, mobile, test coverage, or product polish.
- [ ] Create a `reports/discord-parity.json` entry for every major Discord workflow.
- [ ] Add a proof field for each feature:
  - [ ] Playwright trace or screenshot
  - [ ] API smoke result
  - [ ] permission denial test
  - [ ] Emma Bot run
  - [ ] mobile viewport check
- [ ] Search docs, README, landing pages, and in-app copy for overclaimed wording.
- [ ] Replace "supports X" wording with "X is in progress" unless proof exists.
- [ ] Turn shallow UI surfaces into explicit TODOs instead of hidden assumptions.

Acceptance:

- [ ] No feature is marked complete without evidence.
- [ ] The team can see exactly which Discord workflows are real, shallow, missing, or unknown.
- [ ] The next implementation task is chosen from evidence, not from optimism.

### Milestone 0: Truth Baseline and Regression Harness

Goal: Stop guessing. Build a living gap map and test baseline.

TODO:

- [ ] Create a live parity dashboard JSON at `reports/discord-parity.json`.
- [ ] Add screenshots for Discord-equivalent OpenCorde surfaces under `reports/parity-screenshots/`.
- [ ] Expand `browser_test.py` into named suites:
  - [ ] unauthenticated auth
  - [ ] account settings
  - [ ] server shell
  - [ ] channel chat
  - [ ] forum
  - [ ] voice/stage
  - [ ] admin
  - [ ] permissions denial
- [ ] Add API smoke script for all `/api/v1` route groups.
- [ ] Add migration integrity check: applied migrations must match required tables.
- [ ] Add "no placeholder claims" check for marketing copy.
- [ ] Add Emma Bot smoke script:
  - [ ] post a message
  - [ ] react
  - [ ] create reply/thread
  - [ ] create forum post
  - [ ] RSVP to event
  - [ ] run slash command
- [ ] Make `reports/discord-parity.json` part of every final status.

Acceptance:

- [ ] Browser suite passes.
- [ ] API route smoke passes.
- [ ] Migration/table integrity passes.
- [ ] Every existing "feature-complete" claim is either true or reworded.

### Milestone 1: Discord-Like App Shell

Goal: The product should feel like a client: server rail, channel list, content, member list, user panel, modals, quick switcher.

TODO:

- [ ] Audit desktop and mobile shell dimensions.
- [ ] Make server rail persistent across server routes.
- [ ] Make channel sidebar behavior consistent across text/forum/voice/stage.
- [ ] Add unread badges and mention counts to server rail and channel list.
- [ ] Add channel header actions:
  - [ ] pins
  - [ ] search
  - [ ] threads
  - [ ] notification settings
  - [ ] member list toggle
  - [ ] channel settings
- [ ] Make member list consistently visible/collapsible.
- [ ] Add user panel controls:
  - [ ] status
  - [ ] mute
  - [ ] deafen
  - [ ] settings
  - [ ] profile
- [ ] Add route-safe modal stack for settings/search/pins/threads.
- [ ] Add keyboard shortcuts parity:
  - [ ] Ctrl/Cmd+K quick switcher
  - [ ] Escape closes overlays
  - [ ] Alt+Up/Down channel
  - [ ] Ctrl/Cmd+Shift+M mute
  - [ ] Ctrl/Cmd+Shift+D deafen
- [ ] Playwright screenshot diff for desktop 1440x900, laptop 1280x800, mobile 390x844.

Acceptance:

- [ ] A Discord user can identify server list, channel list, chat, member list, and user controls without instruction.
- [ ] No core workflow requires browser back/forward.
- [ ] Text does not overflow on mobile.

### Milestone 2: Messaging Parity

Goal: Chat should support the standard Discord daily workflow.

TODO:

- [ ] Message grouping by author and timestamp.
- [ ] Date separators.
- [ ] Jump-to-message URL route and scroll restoration.
- [ ] Reply preview with click-to-parent.
- [ ] Edit history indicator.
- [ ] Message context menu parity:
  - [ ] reply
  - [ ] edit
  - [ ] delete
  - [ ] pin
  - [ ] copy text
  - [ ] copy link
  - [ ] mark unread
  - [ ] create thread
  - [ ] apps submenu
  - [ ] report
- [ ] Mentions:
  - [ ] user mention parser
  - [ ] role mention parser
  - [ ] channel mention parser
  - [ ] everyone/here permission check
  - [ ] mention autocomplete
  - [ ] notification event
- [ ] Emoji:
  - [ ] emoji autocomplete
  - [ ] custom emoji rendering
  - [ ] recently used emoji
  - [ ] role/server restrictions
- [ ] Attachments:
  - [ ] upload progress
  - [ ] cancel/retry
  - [ ] spoiler toggle
  - [ ] image lightbox
  - [ ] video/audio preview
  - [ ] download action
- [ ] Poll messages:
  - [ ] DB migration
  - [ ] create poll composer mode
  - [ ] vote endpoint
  - [ ] live result update
  - [ ] close poll
- [ ] Voice messages:
  - [ ] record UI
  - [ ] upload audio
  - [ ] waveform/player
  - [ ] permission flag enforcement
- [ ] Drafts:
  - [ ] local draft per channel/thread/DM
  - [ ] restore on navigation
  - [ ] clear after send
- [ ] Search:
  - [ ] implement syntax filters
  - [ ] search result preview
  - [ ] jump-to-result

Acceptance:

- [ ] Emma can run a 25-message scenario with replies, reactions, attachments, mentions, pins, and search.
- [ ] Two browser sessions see realtime updates without reload.

### Milestone 3: Roles, Permissions, and Access Control

Goal: Role and permission management must be trustworthy, discoverable, and Discord-like.

TODO:

- [ ] Build role list editor:
  - [ ] create role
  - [ ] rename role
  - [ ] color picker
  - [ ] hoist toggle
  - [ ] mentionable toggle
  - [ ] delete role
  - [ ] reorder roles
  - [ ] role icon placeholder
- [ ] Build permission category UI:
  - [ ] general
  - [ ] membership
  - [ ] text
  - [ ] voice
  - [ ] events
  - [ ] apps
  - [ ] moderation
  - [ ] admin
- [ ] Add effective permission inspector:
  - [ ] for role
  - [ ] for member
  - [ ] for channel
  - [ ] explanation of allow/deny/inherit chain
- [ ] Implement channel overwrite matrix:
  - [ ] role overwrites
  - [ ] member overwrites
  - [ ] category inheritance
  - [ ] sync/unsync from category
- [ ] Enforce role hierarchy in backend:
  - [ ] role edit
  - [ ] role delete
  - [ ] member role assignment
  - [ ] kick/ban/timeout
  - [ ] channel overwrite edit
- [ ] Add "View as role/member".
- [ ] Add permission denied UX with reason.
- [ ] Add unit tests for effective permissions.
- [ ] Add Playwright tests for denied actions.

Acceptance:

- [ ] A non-admin cannot grant themselves admin.
- [ ] A lower role cannot moderate a higher role.
- [ ] Channel private access works with one allow and one deny test.

### Milestone 4: Server Settings, Safety, and Moderation

Goal: Server owners should have Discord-like control.

TODO:

- [ ] Server overview:
  - [ ] name
  - [ ] icon
  - [ ] banner
  - [ ] description
  - [ ] system messages channel
  - [ ] rules channel
  - [ ] AFK voice channel
- [ ] Safety setup:
  - [ ] verification level UI
  - [ ] content filter UI
  - [ ] default notification level
  - [ ] explicit media setting
  - [ ] membership screening
- [ ] AutoMod:
  - [ ] keyword rule builder
  - [ ] spam rule builder
  - [ ] mention spam rule
  - [ ] block message action
  - [ ] timeout action
  - [ ] alert channel action
  - [ ] exempt roles/channels
- [ ] Moderation:
  - [ ] member context menu actions
  - [ ] warn records
  - [ ] timeout duration picker
  - [ ] kick/ban reason modal
  - [ ] delete message history option for bans
  - [ ] mod notes
  - [ ] mod queue
- [ ] Audit log:
  - [ ] filters
  - [ ] search
  - [ ] actor/action/date filters
  - [ ] export
- [ ] Reports:
  - [ ] report message
  - [ ] report user
  - [ ] report queue
  - [ ] resolve/escalate

Acceptance:

- [ ] Owner can configure a safe server from one settings area.
- [ ] Moderator can act on a bad message from message context menu.
- [ ] Audit log records every moderation action.

### Milestone 5: Onboarding, Server Guide, Invites, and Discovery

Goal: Joining and orienting should match Discord's community flow.

TODO:

- [ ] Invite preview page:
  - [ ] server icon/name
  - [ ] member count
  - [ ] inviter
  - [ ] expiration/uses
  - [ ] accept/decline
- [ ] Invite management:
  - [ ] max uses
  - [ ] expiration
  - [ ] temporary membership
  - [ ] delete/revoke
  - [ ] vanity code
- [ ] Onboarding:
  - [ ] default channels
  - [ ] question builder
  - [ ] answer options
  - [ ] assign channels by answer
  - [ ] assign roles by answer
  - [ ] preview as new member
- [ ] Server guide:
  - [ ] welcome sign
  - [ ] resource pages
  - [ ] new member checklist
  - [ ] featured channels
- [ ] Discovery:
  - [ ] server categories
  - [ ] tags
  - [ ] language
  - [ ] active member count
  - [ ] search
  - [ ] report server

Acceptance:

- [ ] A new user can join from an invite, complete onboarding, and land in the right channels.

### Milestone 6: Voice, Stage, Video, and Activities

Goal: Voice should be production-usable, not just connected.

TODO:

- [ ] Voice controls:
  - [ ] mute/deafen local
  - [ ] server mute/deafen
  - [ ] move member
  - [ ] disconnect member
  - [ ] per-user volume
  - [ ] input sensitivity
  - [ ] push-to-talk
  - [ ] noise suppression
- [ ] Screen share:
  - [ ] choose screen/window
  - [ ] stream tile
  - [ ] watch stream
  - [ ] stop stream
  - [ ] stream quality selector
- [ ] Stage:
  - [ ] request to speak
  - [ ] approve speaker
  - [ ] move to audience
  - [ ] stage topic
  - [ ] moderator controls
  - [ ] stage event integration
- [ ] Soundboard:
  - [ ] upload sound
  - [ ] play sound with permission
  - [ ] cooldown
  - [ ] volume
- [ ] Recordings/clips:
  - [ ] recording permission
  - [ ] clip creation
  - [ ] clip list
  - [ ] delete/download
- [ ] Activities:
  - [ ] activity launcher placeholder
  - [ ] embedded iframe/web app sandbox policy
  - [ ] permission model

Acceptance:

- [ ] Two users can join voice, mute/deafen, screen share, and leave without stale state.
- [ ] Stage channel can run a speaker/audience scenario.

### Milestone 7: Apps, Bots, Commands, and Webhooks

Goal: OpenCorde should be programmable like Discord.

TODO:

- [ ] App model:
  - [ ] application table
  - [ ] bot user table/link
  - [ ] OAuth scopes
  - [ ] install grants
  - [ ] app owner/developer portal
- [ ] Bot tokens:
  - [ ] create token
  - [ ] rotate token
  - [ ] revoke token
  - [ ] permission intent flags
- [ ] Gateway for bots:
  - [ ] identify
  - [ ] heartbeat
  - [ ] event dispatch
  - [ ] reconnect/resume
  - [ ] rate limits
- [ ] Interactions:
  - [ ] slash command execution
  - [ ] user command execution
  - [ ] message command execution
  - [ ] buttons
  - [ ] select menus
  - [ ] modals
  - [ ] ephemeral responses
  - [ ] deferred responses
- [ ] Command permissions:
  - [ ] per-role
  - [ ] per-user
  - [ ] per-channel
  - [ ] default permissions
- [ ] Webhooks:
  - [ ] execute endpoint parity
  - [ ] avatar/username override
  - [ ] embed payload
  - [ ] history/retry
- [ ] App directory:
  - [ ] list public apps
  - [ ] install flow
  - [ ] app profile
  - [ ] reviews/trust metadata

Acceptance:

- [ ] Emma Bot can be implemented as a real OpenCorde app/bot, not a database/script shortcut.
- [ ] A slash command can return ephemeral and public responses.

### Milestone 8: Notifications, Presence, and Activity

Goal: Users should trust that they will see what matters and not be spammed.

TODO:

- [ ] Presence:
  - [ ] online/idle/dnd/invisible
  - [ ] custom status with expiration
  - [ ] activity status
  - [ ] live updates in member list and DMs
- [ ] Notifications:
  - [ ] server-level all/mentions/nothing
  - [ ] channel override
  - [ ] mute until
  - [ ] suppress everyone/here
  - [ ] suppress role mentions
  - [ ] mobile push
  - [ ] desktop notification
  - [ ] unread badge
  - [ ] recent mentions inbox
- [ ] Privacy:
  - [ ] who can DM
  - [ ] who can friend request
  - [ ] activity visibility
  - [ ] blocked user behavior

Acceptance:

- [ ] Mention notifications fire only when they should.
- [ ] Muted channel/server stays quiet.

### Milestone 9: Mobile and Desktop Client Completion

Goal: OpenCorde should not depend on a desktop browser.

TODO:

- [ ] Desktop:
  - [ ] Windows build
  - [ ] macOS build
  - [ ] Linux build
  - [ ] auto updater
  - [ ] tray behavior
  - [ ] deep links
  - [ ] native notifications
  - [ ] crash logs
- [ ] Mobile:
  - [ ] iOS build
  - [ ] Android build
  - [ ] mobile shell navigation
  - [ ] mobile push
  - [ ] camera/mic permissions
  - [ ] file picker
  - [ ] share target
  - [ ] background reconnect
- [ ] PWA:
  - [ ] install prompt
  - [ ] service worker
  - [ ] offline shell
  - [ ] cache invalidation

Acceptance:

- [ ] Same account can use web, desktop, and mobile without broken state.

### Milestone 10: Federation and Self-Hosted Operations

Goal: The self-hosted advantage should be real.

TODO:

- [ ] Instance setup wizard.
- [ ] SMTP setup/test flow.
- [ ] Storage provider setup/test flow.
- [ ] LiveKit setup/test flow.
- [ ] Backup/restore.
- [ ] Upgrade/migration UI.
- [ ] Federation trust UI:
  - [ ] known peers
  - [ ] allow/block peer
  - [ ] remote user identity
  - [ ] remote DM policy
  - [ ] moderation boundaries
- [ ] Instance health dashboard:
  - [ ] API
  - [ ] DB
  - [ ] Redis
  - [ ] MinIO
  - [ ] LiveKit
  - [ ] SMTP
  - [ ] bridge
  - [ ] background jobs

Acceptance:

- [ ] A non-expert admin can install, verify, monitor, backup, and upgrade an instance.

## Cross-Cutting Technical Debt

TODO:

- [ ] Fix rustfmt drift or document why repo intentionally diverges.
- [ ] Split files over 300 logical lines.
- [ ] Replace placeholder docs in `docs/INDEX.md`.
- [ ] Update marketing copy to say "in progress" where parity is not complete.
- [ ] Add route inventory generated from code.
- [ ] Add endpoint inventory generated from Axum routes.
- [ ] Add permission coverage report.
- [ ] Add DB migration drift report.
- [ ] Add Playwright trace retention on failure.
- [ ] Add seeded demo server reset command.
- [ ] Add seeded test users:
  - [ ] owner
  - [ ] admin
  - [ ] moderator
  - [ ] member
  - [ ] muted member
  - [ ] banned member
  - [ ] bot Emma

## Immediate Next 20 Tasks

These should be done before starting large new features:

- [ ] Update `browser_test.py` into separate suites and keep the current 26/26 as baseline.
- [ ] Add migration integrity check for missing tables like `server_onboarding`.
- [ ] Add `/api/v1/admin/stats` regression test for attachment size totals.
- [ ] Add onboarding endpoint regression test.
- [ ] Add seeded demo/test server reset script.
- [ ] Create Emma Bot account/token strategy.
- [ ] Add "feature claims audit" for landing page and README.
- [ ] Make role permissions panel real, replacing the current guidance placeholder.
- [ ] Implement effective permissions inspector.
- [ ] Add channel/category reorder data model and UI.
- [ ] Add invite preview page.
- [ ] Add message jump links.
- [ ] Add mention parser/autocomplete.
- [ ] Add message grouping/date separators.
- [ ] Add server notification overrides UI.
- [ ] Add AutoMod rule builder.
- [ ] Add voice server mute/deafen controls.
- [ ] Add screen share UI.
- [ ] Add slash command execution path and interaction response model.
- [ ] Add Discord parity JSON report.

## Risk Register

- [ ] Existing docs overstate completeness. Mitigation: feature claims audit.
- [ ] Migrations can drift from DB state. Mitigation: migration/table integrity smoke.
- [ ] Permissions may exist but not be consistently enforced. Mitigation: permission coverage tests.
- [ ] UI can look feature-rich while workflows fail. Mitigation: Emma Bot dogfood scripts.
- [ ] Voice/video features depend on LiveKit setup details. Mitigation: voice health checks and two-client Playwright/manual test.
- [ ] Mobile/Tauri claims may not match real builds. Mitigation: platform build matrix.
- [ ] Discord parity can balloon forever. Mitigation: milestone acceptance criteria and "good enough" definitions.

## Reporting Format

Every work session should end with:

- What changed.
- Which Discord parity area it advances.
- Which checklist items moved.
- Test results.
- Screenshots if UI changed.
- Remaining blocker.
- Next recommended task.

## Bet

The 100 kr bet is not won by claiming parity. It is won only when:

- The parity matrix is mostly green for daily Discord workflows.
- Emma Bot can continuously exercise the live app.
- A new user can complete the path: register -> join server -> onboard -> chat -> voice -> forum -> event -> settings -> notifications.
- A server owner can complete the path: create server -> configure roles -> configure permissions -> create channels -> invite users -> moderate -> integrate bot/webhook -> inspect audit log.

Until then, OpenCorde is a promising Discord alternative, not a Discord replacement.
