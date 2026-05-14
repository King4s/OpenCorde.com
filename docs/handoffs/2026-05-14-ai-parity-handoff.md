# OpenCorde AI Handoff - 2026-05-14

This file is the first context a new AI agent should read before continuing the OpenCorde parity work.

## Repository State

- Repository: `King4s/OpenCorde.com`
- Local path: `/home/mb/opencorde`
- Branch: `main`
- Last pushed functional commit before this update: `5dadd96 feat(admin): surface LiveKit health in instance stats`
- Previous relevant commit: `f9c6a76 fix(voice): speaking indicator, race condition guard, and leave 204`

At the time this handoff was updated, the next task was completed locally and should be committed/pushed with this file: admin LiveKit health proof for `/admin`.

## What Changed Most Recently

Commit `5dadd96` did two things:

- Added LiveKit operational health to `GET /api/v1/admin/stats`.
- Applied `cargo fmt` across the Rust workspace so `cargo fmt --check` passes.

The follow-up task completed after that commit:

- Added `scripts/admin_livekit_health_qa.py`, which creates a short-lived local admin JWT from `.env` without storing it, calls `GET /api/v1/admin/stats`, opens live `/admin` with Playwright, and writes sanitized proof.
- Wrote live proof to `reports/raw/admin-livekit-health-ui.json`.
- Wrote screenshot proof to `reports/parity-screenshots/admin-livekit-health.png`.
- Fixed an admin users panic caused by nullable `users.email` by making `AdminUserRow.email` nullable in Rust/TypeScript and rendering `No email` in the admin user table.
- Rebuilt `client/build`, rebuilt `target/release/opencorde-api`, and restarted `opencorde-api`.

Functional files to inspect first:

- `crates/opencorde-api/src/routes/admin/handlers.rs`
- `crates/opencorde-api/src/routes/admin/types.rs`
- `client/src/lib/api/types.ts`
- `client/src/routes/admin/+page.svelte`
- `client/src/routes/admin/UsersTable.svelte`
- `scripts/admin_livekit_health_qa.py`
- `reports/raw/admin-livekit-health-ui.json`
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

## GitHub Issue Map

Primary source-of-truth issues:

- `#1` Discord parity master plan
- `#2` Milestone -1: brutal audit and claim demotion
- `#4` Messaging parity
- `#5` Roles and permissions parity
- `#7` Voice, video, and stage parity
- `#8` Apps, slash commands, bots, and webhooks parity

Issue `#7` should no longer be treated as needing either the implementation of "LiveKit health to instance report" or the admin UI proof. The implementation is in `5dadd96`; the browser/API proof is in `reports/raw/admin-livekit-health-ui.json` with screenshot `reports/parity-screenshots/admin-livekit-health.png`.

## Current Next TODOs

Recommended order for the next agent:

1. Add Playwright UI proof for private-channel and role-management workflows.
2. Add route inventory JSON generated from Axum route declarations and permission annotations.
3. Start the Playwright parity harness for messaging and roles.
4. Document Emma Bot credentials and test protocol without exposing secrets.
5. Create a two-client voice Playwright/manual checklist for join/leave/mute/deafen/screen-share.

## Exact Next Task Candidate

Best immediate task:

Add Playwright UI proof for private-channel and role-management workflows. Start from the live API smoke in `reports/raw/permission-smoke.json`, then prove the matching user-facing flows and screenshots/traces. Keep `reports/discord-parity.json` and issue `#5` aligned with the result.

## Caution

- Do not mark Discord parity features done just because tables, routes, or UI components exist.
- Do not expose tokens, admin credentials, or Emma Bot credentials in docs, GitHub comments, or reports.
- Keep `reports/discord-parity.json` aligned with GitHub issues when status changes.
- Use `cargo fmt --check` before finalizing Rust changes; the repo is now formatted.
