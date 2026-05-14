# OpenCorde AI Handoff - 2026-05-14

This file is the first context a new AI agent should read before continuing the OpenCorde parity work.

## Repository State

- Repository: `King4s/OpenCorde.com`
- Local path: `/home/mb/opencorde`
- Branch: `main`
- Last pushed functional commit before this update: `52a9902 feat(permissions): gate the 7 likely-real route-inventory gaps`
- Earlier relevant commits: `3c7f70a` transitive recognizer resolution, `a4bff96` recognizer hardening, `d1aecc8` initial route inventory, `231d8e8` permissions UI proof.

At the time this handoff was updated, the next task was completed locally and should be committed/pushed with this file: live-API denial smoke for the 8 newly added gates.

## What Changed Most Recently

The smoke-coverage pass completed after `52a9902`:

- Added 8 nonmember-denial checks to `scripts/permission_smoke.py`:
  - `GET /api/v1/servers/{server_id}` → 403
  - `GET /api/v1/servers/{server_id}/members` → 403
  - `PATCH /api/v1/servers/{server_id}/members/{nonmember_user_id}` → 403 (PATCH-own-nickname on a foreign server)
  - `GET /api/v1/servers/{server_id}/emojis` → 403
  - `GET /api/v1/servers/{server_id}/automod` → 403
  - `PATCH /api/v1/servers/{server_id}/discovery` → 403
  - `PUT /api/v1/channels/{channel_id}/notification-settings` → 403
  - `DELETE /api/v1/channels/{channel_id}/notification-settings` → 403
- Rebuilt `target/release/opencorde-api` and restarted `opencorde-api.service` so the new gates are live.
- Ran `python3 scripts/permission_smoke.py` against `https://opencorde.com` with `OC_MEMBER_EMAIL=browsertest@opencorde.com OC_NONMEMBER_EMAIL=permission-nonmember@opencorde.com OC_LIMITED_EMAIL=permission-limited@opencorde.com`: **52 checks, 0 failures**.
- `reports/raw/permission-smoke.json` regenerated with the expanded coverage.
- `reports/discord-parity.json` evidence_sources updated to record the live proof of all 8 new gates.

Verification:

- `cargo fmt --check` ✓
- `cargo build --release -p opencorde-api` ✓ (deployed; new build serving traffic at opencorde.com)
- `cargo test -p opencorde-api --lib` ✓ — 215 passed, 0 failed
- `python3 scripts/route_inventory.py --fail-on-unresolved` ✓ — 169 routes, 0 unresolved, needs_review 6
- `python3 scripts/permission_smoke.py` (with email env overrides) ✓ — 52 checks, 0 failures

## Operational Notes

- `scripts/permission_smoke.py` default email fixtures use the `@opencorde.local` domain, but the live DB has `@opencorde.com`. Override the email defaults via `OC_MEMBER_EMAIL`, `OC_NONMEMBER_EMAIL`, `OC_LIMITED_EMAIL`. Passwords match the script defaults (`OC_MEMBER_PASSWORD`, `OC_NONMEMBER_PASSWORD`, `OC_LIMITED_PASSWORD`) — set them locally in your environment; do not check fixture passwords into docs.
- After any handler change that affects gates, regenerate the inventory (`python3 scripts/route_inventory.py`) and re-run the smoke. The smoke needs the new code deployed, so the full cycle is: build release → `sudo systemctl restart opencorde-api.service` → smoke.

## GitHub Issue Map

Primary source-of-truth issues:

- `#1` Discord parity master plan
- `#2` Milestone -1: brutal audit and claim demotion
- `#4` Messaging parity
- `#5` Roles and permissions parity
- `#7` Voice, video, and stage parity
- `#8` Apps, slash commands, bots, and webhooks parity

Issue `#5` should treat the route-inventory needs_review list and the permission-smoke results together as the audit trail: any new endpoint without a recognized gate either lands in needs_review (caught by inventory) or fails non-member denial (caught by smoke).

## Current Next TODOs

Recommended order for the next agent:

1. Start the Playwright parity harness for messaging and roles (Issue `#4`, `#5`).
2. Document Emma Bot credentials and test protocol without exposing secrets (Issue `#8`).
3. Two-client voice Playwright/manual checklist for join/leave/mute/deafen/screen-share (Issue `#7`).
4. (Optional) Add a `caller_implicit` path_kind classifier in `scripts/route_inventory.py` so the 6 remaining accepted-by-design entries drop out of needs_review.
5. Broaden the permission matrix to owner/admin/mod/member/muted/banned workflows.

## Exact Next Task Candidate

Best immediate task:

Begin the Playwright parity harness for messaging. Suggested first scope: a new `scripts/messaging_ui_qa.py` that proves send/edit/delete/reply/react/pin against a live channel as the `browsertest_user` fixture, mirroring the structure of `scripts/permissions_ui_qa.py`. Write proof to `reports/raw/messaging-ui-proof.json` and screenshots to `reports/parity-screenshots/messaging-ui/`.

## Caution

- Do not mark Discord parity features done just because tables, routes, or UI components exist.
- Do not expose tokens, admin credentials, or Emma Bot credentials in docs, GitHub comments, or reports.
- Keep `reports/discord-parity.json` aligned with GitHub issues when status changes.
- `reports/raw/route-inventory.json` is generated. Re-run `python3 scripts/route_inventory.py` after any router/handler change instead of editing it by hand.
- The `require_server_perm(.., Permissions::VIEW_CHANNEL)` calls added to get_server / list_emojis / list_members / update_member double as membership checks because `VIEW_CHANNEL` is in `default_everyone`. If `default_everyone` ever changes, audit these handlers.
- Use `cargo fmt --check` before finalizing Rust changes; the repo is now formatted.
