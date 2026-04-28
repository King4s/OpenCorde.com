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

## Still Open

- Channel overwrite computation still needs closer Discord parity:
  - first-class `@everyone` overwrite behavior
  - aggregate role deny/allow precedence
  - member overwrite precedence tests

- Role hierarchy is still not implemented:
  - assigning roles
  - editing roles
  - deleting roles
  - moderation targets
  - self-escalation through powerful roles

- Search still needs authorization filtering before returning results.

- More channel-scoped routes still need audit/fixes:
  - forum
  - pins
  - reactions list/remove
  - uploads
  - recordings
  - soundboard
  - E2EE/read-state
  - events

- Webhook response shape still exposes token in list responses. This is now restricted to `MANAGE_WEBHOOKS`, but token exposure should still be hardened so tokens are only shown on creation/regeneration.

- Stage permissions still need a deeper model:
  - `CONNECT`
  - `SPEAK`
  - `REQUEST_TO_SPEAK`
  - `MUTE_MEMBERS`
  - `MOVE_MEMBERS`
  - stage speaker/audience alignment with LiveKit publish grants

## Verification

- `cargo check -p opencorde-api`: passed.
- `cargo build --release -p opencorde-api`: passed.
- `systemctl restart opencorde-api`: completed.
- `https://opencorde.com/api/v1/health`: OK.
- `python3 browser_test.py`: 26 passed, 0 failed.
- `scripts/public_qa.py --fail-on-issues`: passed.
- `scripts/permission_smoke.py`: passed against live API.

Permission smoke coverage:

- non-member cannot create server invite: 403
- non-member cannot list server invites: 403
- non-member cannot read onboarding: 403
- non-member cannot list channel threads: 403
- non-member cannot list channel webhooks: 403

## Next Recommended Fixes

- Add permission regression tests for non-member denial on invite/list/webhook/thread/voice routes.
- Add route inventory JSON generated from Axum route declarations and permission annotations.
- Add `MANAGE_MESSAGES` support for moderator message delete.
- Add `ATTACH_FILES` and `SEND_MESSAGES` checks to uploads.
- Add `PIN_MESSAGES` or `MANAGE_MESSAGES` checks to pins.
- Add search result filtering by effective channel visibility.
- Implement role hierarchy and protected target rules.
