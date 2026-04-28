# OpenCorde Discord Foundation Audit

Date: 2026-04-28
Scope: read-only audit of current Discord-like foundations.
Repository: `King4s/OpenCorde.com`

## Summary

OpenCorde is not an empty stub. It has real schema, repositories, API routes, client stores, and UI components for many Discord-like concepts.

It is also not Discord parity. The recurring gap is end-to-end product completeness: permission boundaries are uneven, realtime propagation is partial, UI exposes only subsets of expected workflows, and most features are not proven by browser/API/mobile/Emma evidence.

Default status for all audited areas: `shallow_needs_proof`.

## Messaging

Observed evidence:

- `messages` table supports content, attachment metadata, replies, edits, pins, read state, and thread-related data.
- API paths exist for send/list/edit/delete/typing/read-state/reactions/pins/threads.
- Send path enforces `SEND_MESSAGES`, slowmode, verification level, automod, and emits websocket events.

Likely gaps:

- Edit/delete behavior appears author-focused and needs `MANAGE_MESSAGES` proof for moderators.
- `READ_MESSAGE_HISTORY`, thread-specific permissions, and typing access need consistent enforcement proof.
- Attachment metadata is JSON-shaped and not yet proven as a full upload ownership/deletion lifecycle.
- Rich Discord semantics are not proven: embeds, mentions, flags, system messages, message links, local drafts, upload progress.
- Realtime behavior for updates/deletes/reactions/pins/read-state needs multi-client proof.

Priority TODOs:

- Enforce and test `MANAGE_MESSAGES`, `READ_MESSAGE_HISTORY`, thread permissions, and typing access.
- Add Playwright/API coverage for send/edit/delete/reply/react/pin/upload/search.
- Add robust mention parsing/counting and unread behavior.
- Add moderation/audit coverage for non-author delete and bulk operations.
- Add Emma Bot chat script.

## Roles, Permissions, and Channel Overrides

Observed evidence:

- Discord-style permission bitfield exists in `opencorde-core`.
- `compute_permissions` exists.
- Roles CRUD, member role assignment APIs, and channel override routes exist.
- Several backend routes call server or channel permission checks.
- Client has role management and channel permissions components.

Likely gaps:

- UI is not proven as a true Discord-like permission matrix.
- Role hierarchy checks need proof for assignment, edits, moderation, and member actions.
- `@everyone` behavior appears fallback-like and needs first-class proof.
- Permission checks are not proven across every route group.
- Channel overwrite precedence needs direct tests.

Priority TODOs:

- Build an effective permission inspector per user/channel.
- Add first-class `@everyone` behavior and auditability.
- Add role hierarchy enforcement.
- Audit every route for required permissions.
- Add integration tests for owner/admin/mod/member/muted/banned users.

## Onboarding and Server Guide

Observed evidence:

- `server_onboarding` table exists after manual repair.
- GET/PUT onboarding API exists.
- Settings panel can toggle onboarding and save a welcome message.

Likely gaps:

- New-member join -> onboarding -> selected channels workflow is not proven.
- Prompt schema is opaque JSON and needs validation.
- Role/channel assignment from onboarding is not proven.
- Management appears owner-gated instead of permission-gated.
- Per-member completion/skips are missing.

Priority TODOs:

- Add migration/table integrity smoke test.
- Implement invite -> join -> onboarding -> selected channels Playwright flow.
- Define validated prompt schema.
- Store per-member onboarding completion.
- Add Emma Bot onboarding script.

## Voice, Video, and Stage

Observed evidence:

- LiveKit token generation exists.
- `voice_states` repo/routes exist.
- Client has voice store/components, mic/camera selection, video tracks, and E2EE hook.
- Stage schema, sessions, participants, and client components exist.

Likely gaps:

- Voice join validates channel type but needs proof for `CONNECT`, `SPEAK`, `STREAM`, server membership, mute/deafen, and move permissions.
- LiveKit token grants appear broad and need alignment with effective permissions.
- Stage speaker/audience state must be enforced through token grants, not just UI/DB state.
- Server-side voice/stage state events and stale cleanup need proof.

Priority TODOs:

- Enforce voice/stage permission gates on routes and token grants.
- Align stage speaker/audience state with LiveKit publish rights.
- Emit and consume server-side voice/stage state events.
- Add stale voice-state cleanup.
- Prove two-user voice/stage flows.

## Slash Commands, Apps, Bots, and Webhooks

Observed evidence:

- Slash command table/repo/routes exist.
- Client command autocomplete/dispatch exists.
- Webhook table/repo/routes and client manager exist.
- Discord bridge routes exist.

Likely gaps:

- Slash commands are closer to outbound HTTP hooks than Discord's interaction model.
- Command options, bot/application identity, signatures, install scopes, context commands, buttons, selects, modals, ephemeral responses, and deferred responses are not proven.
- Webhook management needs `MANAGE_WEBHOOKS` proof.
- Webhook token exposure needs hardening.
- Realtime/audit behavior for webhook execution needs proof.

Priority TODOs:

- Add `USE_APPLICATION_COMMANDS` and `MANAGE_WEBHOOKS` permission gates.
- Introduce bot/application identity.
- Add command option schema and interaction response lifecycle.
- Stop exposing webhook tokens in normal list responses.
- Add Emma Bot command/webhook script.

## Friends and DMs

Observed evidence:

- Relationships schema supports pending/accepted/blocked.
- Friend request/accept/list/block/search routes and client page/store exist.
- DM tables/routes support local one-to-one DMs, membership checks, message list/send, and websocket `DmMessageCreate`.
- Federated DM path exists.

Likely gaps:

- Relationship uniqueness appears directional and may allow duplicate reciprocal states.
- Blocking is not proven across DM open/send/friend request paths.
- Group DMs are mentioned by schema comments but not proven in behavior.
- DM edit/delete/reactions/attachments/read receipts/unread behavior is not parity-proven.
- Realtime friend request/accept/block and presence integration need proof.

Priority TODOs:

- Normalize relationship constraints for unordered user pairs.
- Enforce block/friend state consistently.
- Add DM edit/delete/read receipt/unread behavior.
- Decide group DM semantics or remove misleading comments.
- Add friends/DM Playwright suite.

## Current Conclusion

OpenCorde should be described as a promising self-hosted Discord-style platform in active development.

It should not be described as feature-complete or Discord-parity until each core workflow has current evidence in `reports/discord-parity.json`.
