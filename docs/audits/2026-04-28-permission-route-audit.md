# OpenCorde Permission Route Audit

Date: 2026-04-28
Scope: backend route permission enforcement.
Repository: `King4s/OpenCorde.com`

## Summary

The permission system has useful primitives: Discord-style bitflags, owner bypass, administrator bypass, role-based base permissions, and channel overwrites.

The biggest discovered defect was that `compute_base_perms` granted `Permissions::default_everyone()` when a user had no role rows. Because that function did not first prove server membership, non-members could be treated like ordinary members for routes using `require_server_perm` or `require_channel_perm`.

This audit started fixing high-risk gaps, but permissions are still not parity-complete.

## Fixed in First Pass

- `permission_check.rs`
  - `compute_base_perms` now requires an actual `server_members` row before granting any default permissions.
  - Non-members now receive `403 Forbidden` instead of `default_everyone()`.

- `voice`
  - `join_voice` now requires channel `CONNECT`.
  - `get_participants` now requires channel `CONNECT`.
  - `get_livekit_token` rechecks channel `CONNECT`.

- `webhooks`
  - Create/list/delete now require channel `MANAGE_WEBHOOKS`.
  - Delete is no longer limited to creator-only; authorized webhook managers can delete.
  - List responses no longer expose webhook execution tokens; tokens are returned only at creation time.

- `slash_commands`
  - Create/delete now require server `MANAGE_SERVER` instead of owner-only.
  - List now requires server membership through `VIEW_CHANNEL`.
  - Dispatch now requires `USE_APPLICATION_COMMANDS` and `SEND_MESSAGES` in the target channel.

- `onboarding`
  - Read now requires server membership through `VIEW_CHANNEL`.
  - Update now requires `MANAGE_SERVER` instead of owner-only.

- `audit_log`
  - Read now requires `VIEW_AUDIT_LOG` instead of owner-only.

- `threads`
  - Create now requires `CREATE_PUBLIC_THREADS`.
  - List/get now require `VIEW_CHANNEL`.
  - List messages now requires `VIEW_CHANNEL` plus `READ_MESSAGE_HISTORY`.
  - Send in thread now requires `SEND_MESSAGES_IN_THREADS`.

- `uploads`
  - Attachment upload now requires `VIEW_CHANNEL`, `SEND_MESSAGES`, and `ATTACH_FILES`.

- `pins`
  - Listing pins now requires `VIEW_CHANNEL`.
  - Pin/unpin now requires `PIN_MESSAGES`.

- `reactions`
  - Add reaction now requires `VIEW_CHANNEL` plus `ADD_REACTIONS`.
  - Remove reaction now requires `VIEW_CHANNEL`.
  - List reactions now requires `VIEW_CHANNEL`.

- `messages`
  - Message authors can still delete their own messages.
  - Non-author message deletion now requires channel `VIEW_CHANNEL` plus `MANAGE_MESSAGES`.
  - Typing indicators now require channel `VIEW_CHANNEL` plus `SEND_MESSAGES` before broadcasting.

- `search`
  - Message search now filters every hit through channel `VIEW_CHANNEL` before returning content.
  - Search over-fetches before filtering so authorized users still receive useful result counts.

- `forum`
  - Listing posts and post detail now require channel `VIEW_CHANNEL`.
  - Creating posts/replies now requires channel `VIEW_CHANNEL` plus `SEND_MESSAGES`.
  - Deleting posts/replies now requires channel visibility before author/owner checks.

- `soundboard`
  - Listing and playing sounds now require server `VIEW_CHANNEL`.
  - Creating/deleting sounds now require `MANAGE_GUILD_EXPRESSIONS` instead of owner-only.

- `recordings`
  - Listing recordings now requires channel `VIEW_CHANNEL`.
  - Start/stop recording now require channel `VIEW_CHANNEL` plus `MANAGE_CHANNELS` instead of owner-only.

- `E2EE groups`
  - Init/update now require channel `VIEW_CHANNEL` plus `SEND_MESSAGES`.
  - Welcome fetch now requires channel `VIEW_CHANNEL`.
  - Init now validates every welcome recipient against channel `VIEW_CHANNEL` before writing group state or welcome rows.

- `read_state`
  - Channel ack now requires `VIEW_CHANNEL`.
  - Read-state list filters stale or unauthorized channel entries before returning them.

- `E2EE key packages`
  - Consuming another user's key package now requires a `channel_id` scope.
  - The requester and target user must both have channel `VIEW_CHANNEL` for that scoped channel.
  - Users may still consume their own available package.

- `events`
  - Listing server events now requires server `VIEW_CHANNEL` and filters channel-bound events by channel visibility.
  - Creating events now requires `CREATE_EVENTS`; channel-bound events also require channel visibility and same-server validation.
  - Reading, RSVP, and un-RSVP now require event visibility.
  - Updating/deleting events now require creator status or server `MANAGE_EVENTS`.

- `roles`
  - Listing server roles and member roles now requires server membership through `VIEW_CHANNEL`.
  - Role update/delete/assign/unassign now enforce hierarchy: non-owner actors can only manage roles below their highest role, can only change members below their highest role, and cannot move roles to their own or higher position.
  - Role create/update now rejects permission bits the actor does not effectively hold, with owner/administrator resolving to all known bits.
  - Added batch role reorder endpoint (`PATCH /servers/{server_id}/roles`) with hierarchy validation before applying position changes.

- `moderation`
  - Ban, timeout, timeout removal, and kick now enforce target hierarchy.
  - Non-owner moderators cannot target the server owner or members with equal/higher top role position.

- `permission_compute`
  - Channel overwrites now follow Discord precedence layers: `@everyone` first, aggregate matching role denies/allows next, member overwrite last.
  - Added core tests for aggregate role overwrite behavior and `@everyone` precedence.
  - Added member-specific deny precedence regression coverage.

- `channels`
  - Server channel listing now requires server membership through `VIEW_CHANNEL`.
  - Channel listing filters every returned channel through channel `VIEW_CHANNEL`, so private channels are hidden from members denied by overwrites.

- `channel_overrides`
  - Listing permission overrides now requires channel `VIEW_CHANNEL`, preventing private-channel override metadata leaks to ordinary server members.
  - Upsert/delete now require channel `MANAGE_CHANNELS`, so channel overwrites affect override management consistently.

- `voice / LiveKit`
  - Voice join and fresh LiveKit token routes still require channel `CONNECT`.
  - LiveKit publish grants now depend on effective channel `SPEAK`; users with `CONNECT` but denied `SPEAK` receive subscribe-only tokens.
  - Stage RTC publish grants now require both effective channel `SPEAK` and current stage participant role `speaker`; `audience` participants receive subscribe-only tokens.

- `stage`
  - Stage detail and join now require channel `CONNECT`.
  - Starting a stage requires channel `CONNECT`, `SPEAK`, and `MUTE_MEMBERS`.
  - Ending a stage allows the starter or users with channel `MUTE_MEMBERS`.
  - Raising hand requires an active session, existing participant row, channel `CONNECT`, and `REQUEST_TO_SPEAK`.
  - Speaker promotion/demotion requires the starter or channel `MUTE_MEMBERS`; promotion also verifies the target has channel `SPEAK`.

- `effective permissions`
  - Added manager-gated server and channel effective permission inspector endpoints.
  - Server inspector returns role-derived effective permissions with owner/administrator resolving to all known bits.
  - Channel inspector applies the same overwrite computation used by route enforcement and returns both bitfield and permission names.

## Still Open

- Channel overwrite computation still needs Playwright UI proof for private-channel deny/allow workflows.

- Role hierarchy still needs deeper Discord parity:
  - UI proof for batch role reordering and effective permission inspector

- Stage permissions still need a deeper model:
  - `MOVE_MEMBERS`
  - realtime/client proof that promoted or demoted stage users refresh RTC grants correctly

## Verification

- `cargo check -p opencorde-api`: passed.
- `cargo build --release -p opencorde-api`: passed.
- `systemctl restart opencorde-api`: completed.
- `https://opencorde.com/api/v1/health`: OK.
- `python3 browser_test.py`: 26 passed, 0 failed.
- `scripts/public_qa.py --fail-on-issues`: passed.
- `scripts/permission_smoke.py`: passed against live API.
- `scripts/schema_smoke.py`: passed against live DB; 45 expected tables, 314 expected columns, and 51 applied migrations.

Permission smoke coverage:

- non-member cannot create server invite: 403
- non-member cannot list server invites: 403
- non-member cannot read onboarding: 403
- non-member cannot list channel threads: 403
- non-member cannot list channel webhooks: 403
- non-member cannot list channel pins: 403
- non-member cannot pin channel message: 403
- non-member cannot list message reactions: 403
- non-member search cannot see private server messages: 200 with empty results
- non-member cannot list/create/delete/play soundboard entries: 403
- non-member cannot list channel recordings: 403
- non-member cannot ack channel read state: 403
- non-member cannot initialize/fetch/update E2EE group state: 403
- non-member cannot send typing indicator: 403
- non-member cannot list/create server events: 403
- non-member cannot list server roles: 403
- non-member cannot consume channel-scoped user key package: 403
- private channel hidden from member without allowed role: 200 without channel in list
- private channel messages denied without allowed role: 403
- private channel scoped key package denied without view: 403
- private channel E2EE init denies welcome for hidden member: 403
- private channel visible with allowed role: 200 with channel in list
- private channel messages allowed with allowed role: 200
- voice member with `CONNECT` but denied `SPEAK` receives join LiveKit token with `canPublish=false`
- voice member with `CONNECT` but denied `SPEAK` receives fresh LiveKit token with `canPublish=false`
- stage detail and join denied without `CONNECT`: 403
- stage hand raise denied without `REQUEST_TO_SPEAK`: 403
- stage speaker promotion denied when target lacks `SPEAK`: 403
- stage audience with `SPEAK` receives subscribe-only RTC token with `canPublish=false`
- stage speaker with `SPEAK` receives publish RTC token with `canPublish=true`
- demoted stage audience receives refreshed subscribe-only RTC token with `canPublish=false`
- role batch reorder cannot move a role to the actor's own position: 403
- role manager cannot assign a role to a same-position target: 403
- role manager cannot remove a role from a same-position target: 403
- server effective permission inspector returns owner permissions: 200
- channel effective permission inspector returns channel permissions: 200
- moderator can delete another user's message: 204
- role hierarchy smoke: manager cannot create/edit roles with unheld permission bits, move a lower role to equal position, change roles on same-position targets, or delete own top role: 403
- moderation hierarchy smoke: moderator cannot ban, timeout, or kick a same-position target: 403

## Next Recommended Fixes

- Add Playwright UI proof for private-channel and role-management workflows.
- Add route inventory JSON generated from Axum route declarations and permission annotations.
- Add LiveKit operational health to instance reporting.
