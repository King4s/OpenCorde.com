# OpenCorde AI Handoff - 2026-05-14

This file is the first context a new AI agent should read before continuing the OpenCorde parity work.

## Repository State

- Repository: `King4s/OpenCorde.com`
- Local path: `/home/mb/opencorde`
- Branch: `main`
- Last pushed functional commit before this update: `3c7f70a audit(routes): transitive helper resolution + named-gate recognizers`
- Earlier relevant commits: `a4bff96` recognizer hardening, `d1aecc8` initial route inventory, `231d8e8` permissions UI proof, `ae6ed3b` LiveKit health proof.

At the time this handoff was updated, the next task was completed locally and should be committed/pushed with this file: gates added for the 7 likely-real gaps surfaced by the route inventory. `needs_review` is now 6, all accepted-by-design.

## What Changed Most Recently

The permission gate pass completed after `3c7f70a`:

- `crates/opencorde-api/src/routes/automod/handlers.rs::list_rules` — added `permission_check::require_server_perm(.., Permissions::MANAGE_SERVER)`. The "no permission check needed for viewing rules" comment was misleading; rule keyword lists are sensitive. Behavior now matches POST/PATCH/DELETE on the same surface.
- `crates/opencorde-api/src/routes/servers/handlers/crud.rs::get_server` — added `require_server_perm(.., Permissions::VIEW_CHANNEL)`. `VIEW_CHANNEL` is part of `Permissions::default_everyone()`, so this works as a membership test: owner passes, members pass, non-members get Forbidden.
- `crates/opencorde-api/src/routes/emojis.rs::list_emojis` — same membership test; removed the now-unused `verify_server_exists` helper since `require_server_perm` covers existence + membership.
- `crates/opencorde-api/src/routes/members.rs::list_members` and `::update_member` — same membership test; `update_member` retains its self-only check via `target_user_id != auth.user_id`.
- `crates/opencorde-api/src/routes/discovery.rs::update_discovery` — refactored to use `helpers::check_server_owner(auth.user_id, owner_id)?` (same gate, but now recognized by the inventory's named-gate regex).
- `crates/opencorde-api/src/routes/notification_settings.rs::set_setting` and `::reset_setting` — added `require_channel_perm(.., Permissions::VIEW_CHANNEL)`. Now you can only set notification preferences for channels you can see.

Verification:

- `cargo fmt --check` passes.
- `cargo check -p opencorde-api` passes.
- `cargo test -p opencorde-api --lib` passes: 215 passed, 0 failed.
- `python3 scripts/route_inventory.py --fail-on-unresolved` reports 169 routes, 0 unresolved, `needs_review` 6 (down from 14).

The remaining 6 `needs_review` entries are all accepted-by-design:

- `GET /api/v1/servers` — caller's own server list, scoped server-side.
- `POST /api/v1/servers` — any authed user can create a new server (they become owner).
- `GET /api/v1/users/{id}` — Discord-style public profile fetch, open to any authed user.
- `POST /api/v1/voice/leave` and `PATCH /api/v1/voice/state` — own voice state.
- `DELETE /api/v1/channels/{channel_id}/stage/leave` — own stage state; leave is a no-op for non-participants.

None of these are handler-side gaps. They are intentional auth-only endpoints whose URL shape the inventory's `path_kind` heuristic doesn't yet classify. If you want a clean zero, add a `caller_implicit` `path_kind` in `scripts/route_inventory.py` covering `/api/v1/voice/`, `/api/v1/servers` (exact match), the `stage/leave` shape, and `GET /api/v1/users/{id}`. Otherwise leave them documented as accepted in `reports/discord-parity.json`.

Functional files to inspect first:

- `crates/opencorde-api/src/routes/{automod,emojis,members,discovery,notification_settings}/...` (the 7 gates).
- `crates/opencorde-api/src/routes/servers/handlers/crud.rs` (get_server gate).
- `scripts/route_inventory.py` (regenerator).
- `reports/raw/route-inventory.json` (regenerated; new `by_permission` totals: server:MANAGE_SERVER 4→5, server:VIEW_CHANNEL 8→15, channel:VIEW_CHANNEL 30→36).
- `reports/discord-parity.json` (roles_permissions area).

## GitHub Issue Map

Primary source-of-truth issues:

- `#1` Discord parity master plan
- `#2` Milestone -1: brutal audit and claim demotion
- `#4` Messaging parity
- `#5` Roles and permissions parity
- `#7` Voice, video, and stage parity
- `#8` Apps, slash commands, bots, and webhooks parity

Issue `#5` should reference `reports/raw/route-inventory.json` (`needs_review_routes`) — now 6 routes, all accepted. The previous likely-real-gap list is closed.

## Current Next TODOs

Recommended order for the next agent:

1. **Add API denial smoke tests** in `scripts/permission_smoke.py` covering the 8 new gates (one per endpoint, hit it as a non-member, assert 403/404). Patterns to copy from the existing private-channel smoke section.
2. (Optional) Add a `caller_implicit` path_kind classifier to drive `needs_review` to 0 without hand-editing.
3. Start the Playwright parity harness for messaging and roles (Issue `#4`).
4. Document Emma Bot credentials and test protocol without exposing secrets (Issue `#8`).
5. Two-client voice Playwright/manual checklist (Issue `#7`).
6. Add the broader permission matrix for owner/admin/mod/member/muted/banned workflows (Issue `#5` continuation).

## Exact Next Task Candidate

Best immediate task:

Extend `scripts/permission_smoke.py` with a section that creates a server + a non-member fixture, then hits each of the 8 newly-gated endpoints and asserts Forbidden. Write the result into `reports/raw/permission-smoke.json` and reference it from `discord-parity.json`. Confirm the new section keeps the rest of permission_smoke.py green.

## Caution

- Do not mark Discord parity features done just because tables, routes, or UI components exist.
- Do not expose tokens, admin credentials, or Emma Bot credentials in docs, GitHub comments, or reports.
- Keep `reports/discord-parity.json` aligned with GitHub issues when status changes.
- `reports/raw/route-inventory.json` is generated. Re-run `python3 scripts/route_inventory.py` after any router/handler change instead of editing it by hand.
- The new `require_server_perm(.., Permissions::VIEW_CHANNEL)` calls double as membership checks because `VIEW_CHANNEL` is in `default_everyone`. If the default-everyone bitmask is ever changed, audit these handlers — they may need to switch to an explicit membership helper.
- Use `cargo fmt --check` before finalizing Rust changes; the repo is now formatted.
