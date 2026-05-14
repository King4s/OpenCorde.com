# OpenCorde AI Handoff - 2026-05-14

This file is the first context a new AI agent should read before continuing the OpenCorde parity work.

## Repository State

- Repository: `King4s/OpenCorde.com`
- Local path: `/home/mb/opencorde`
- Branch: `main`
- Last pushed functional commit before this update: `ae6ed3b test(admin): prove LiveKit health dashboard`
- Previous relevant commit: `f9c6a76 fix(voice): speaking indicator, race condition guard, and leave 204`

At the time this handoff was updated, the next task was completed locally and should be committed/pushed with this file: Playwright UI proof for private-channel and role-management workflows.

## What Changed Most Recently

Commit `5dadd96` did two things:

- Added LiveKit operational health to `GET /api/v1/admin/stats`.
- Applied `cargo fmt` across the Rust workspace so `cargo fmt --check` passes.

The admin LiveKit follow-up task completed after that commit:

- Added `scripts/admin_livekit_health_qa.py`, which creates a short-lived local admin JWT from `.env` without storing it, calls `GET /api/v1/admin/stats`, opens live `/admin` with Playwright, and writes sanitized proof.
- Wrote live proof to `reports/raw/admin-livekit-health-ui.json`.
- Wrote screenshot proof to `reports/parity-screenshots/admin-livekit-health.png`.
- Fixed an admin users panic caused by nullable `users.email` by making `AdminUserRow.email` nullable in Rust/TypeScript and rendering `No email` in the admin user table.
- Rebuilt `client/build`, rebuilt `target/release/opencorde-api`, and restarted `opencorde-api`.

The permission UI follow-up task completed after `ae6ed3b`:

- Added `scripts/permissions_ui_qa.py`, which creates temporary live DB fixtures, proves browser behavior with Playwright, writes sanitized proof, and cleans fixtures in `finally`.
- Wrote live proof to `reports/raw/permissions-ui-proof.json`.
- Wrote screenshots under `reports/parity-screenshots/permissions-ui/`.
- Fixed `ChannelPermissionsTab` so it fetches roles before rendering override labels; without this, override rows showed fallback `Role <id>` labels.
- Added stable `data-role-name` hooks to `RolesPanel` rows so role create/rename/delete proof targets the specific UI-created role.
- Rebuilt `client/build` after the frontend changes.

Functional files to inspect first:

- `crates/opencorde-api/src/routes/admin/handlers.rs`
- `crates/opencorde-api/src/routes/admin/types.rs`
- `client/src/lib/api/types.ts`
- `client/src/routes/admin/+page.svelte`
- `client/src/routes/admin/UsersTable.svelte`
- `client/src/lib/components/modals/ChannelPermissionsTab.svelte`
- `client/src/routes/servers/[serverId]/settings/panels/RolesPanel.svelte`
- `scripts/admin_livekit_health_qa.py`
- `scripts/permissions_ui_qa.py`
- `reports/raw/admin-livekit-health-ui.json`
- `reports/raw/permissions-ui-proof.json`
- `reports/discord-parity.json`
- `docs/audits/2026-04-28-permission-route-audit.md`

Large Rust diffs in the same commit are formatting-only unless they touch the admin LiveKit health code above.

## Verification Already Run

These commands passed after the latest changes:

```bash
cargo fmt --check
cargo check -p opencorde-api
cargo test -p opencorde-api admin --quiet
cd client && pnpm check
git diff --check
python3 -m json.tool reports/discord-parity.json >/dev/null
python3 -m py_compile scripts/admin_livekit_health_qa.py
python3 scripts/admin_livekit_health_qa.py --fail-on-issues
python3 -m py_compile scripts/permissions_ui_qa.py
python3 scripts/permissions_ui_qa.py --fail-on-issues
```

The admin LiveKit health proof ran against `https://opencorde.com` and passed with:

- API status `200`
- `livekit_health.ok=true`
- local LiveKit status `200`
- public proxy status `200`
- `/admin` status `200`
- LiveKit Health panel visible
- Local and Public Proxy rows visible
- no browser console errors, page errors, failed requests, or token leakage in page HTML

The permission UI proof ran against `https://opencorde.com` and passed with:

- private channel hidden from the limited user before role assignment
- same channel visible/openable after assigning the allowed role
- channel permissions modal renders the allowed role name and `View Channel` controls
- role create, rename, and delete all work from server settings
- no browser console errors, page errors, or non-ignored failed requests
- temporary `private-ui-*`, `allow-ui-*`, and `managed-ui-*` fixtures removed after the run

## GitHub Issue Map

Primary source-of-truth issues:

- `#1` Discord parity master plan
- `#2` Milestone -1: brutal audit and claim demotion
- `#4` Messaging parity
- `#5` Roles and permissions parity
- `#7` Voice, video, and stage parity
- `#8` Apps, slash commands, bots, and webhooks parity

Issue `#7` should no longer be treated as needing either the implementation of "LiveKit health to instance report" or the admin UI proof. The implementation is in `5dadd96`; the browser/API proof is in `reports/raw/admin-livekit-health-ui.json` with screenshot `reports/parity-screenshots/admin-livekit-health.png`.

Issue `#5` should no longer be treated as needing first proof for "private-channel and role-management UI workflows"; first proof is in `reports/raw/permissions-ui-proof.json` with screenshots under `reports/parity-screenshots/permissions-ui/`. It still needs a broader permission matrix and UI proof for batch role reordering/effective permission inspector workflows.

## Current Next TODOs

Recommended order for the next agent:

1. Add route inventory JSON generated from Axum route declarations and permission annotations.
2. Start the Playwright parity harness for messaging and roles.
3. Broaden permission UI proof to owner/admin/mod/member/muted/banned workflows.
4. Document Emma Bot credentials and test protocol without exposing secrets.
5. Create a two-client voice Playwright/manual checklist for join/leave/mute/deafen/screen-share.

## Exact Next Task Candidate

Best immediate task:

Add route inventory JSON generated from Axum route declarations and permission annotations. Keep `reports/discord-parity.json` and the GitHub issue map aligned with the result.

## Caution

- Do not mark Discord parity features done just because tables, routes, or UI components exist.
- Do not expose tokens, admin credentials, or Emma Bot credentials in docs, GitHub comments, or reports.
- Keep `reports/discord-parity.json` aligned with GitHub issues when status changes.
- Use `cargo fmt --check` before finalizing Rust changes; the repo is now formatted.
