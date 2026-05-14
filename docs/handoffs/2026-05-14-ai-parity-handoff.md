# OpenCorde AI Handoff - 2026-05-14

This file is the first context a new AI agent should read before continuing the OpenCorde parity work.

## Repository State

- Repository: `King4s/OpenCorde.com`
- Local path: `/home/mb/opencorde`
- Branch: `main`
- Last pushed functional commit before this update: `231d8e8 test(permissions): prove private channel and role UI`
- Previous relevant commit: `ae6ed3b test(admin): prove LiveKit health dashboard`

At the time this handoff was updated, the next task was completed locally and should be committed/pushed with this file: route inventory generator + first inventory artifact.

## What Changed Most Recently

The route inventory task completed after `231d8e8`:

- Added `scripts/route_inventory.py`, which walks the API module graph from `crates/opencorde-api/src/routes/mod.rs` and `crates/opencorde-api/src/ws/handler/mod.rs`, parses every `.route(...)` call, resolves handler references to their fn definitions, and extracts permission gates (`require_server_perm`, `require_channel_perm`, `is_admin`, `check_verification_level`, role-hierarchy and rate-limit helpers) plus auth-class (admin / user / public).
- Wrote first inventory to `reports/raw/route-inventory.json`. Summary at the time of this handoff: 169 endpoint registrations across 126 unique paths, 0 unresolved handlers, by auth class admin=7 / user=144 / public=18.
- Updated `reports/discord-parity.json` to register the inventory under `evidence_sources`, add it to `proof_required` for the roles/permissions area, surface the `60 auth-only routes` audit list as a gap and as `next_session_focus`.

The earlier handoff baseline (admin LiveKit health + permission UI proof from commits `5dadd96` / `ae6ed3b` / `231d8e8`) is unchanged. See git log for those changes.

Functional files to inspect first:

- `scripts/route_inventory.py`
- `reports/raw/route-inventory.json`
- `reports/discord-parity.json`
- `crates/opencorde-api/src/routes/mod.rs` (entry point for the module walk)
- `crates/opencorde-api/src/routes/permission_check.rs` (the helpers the inventory looks for)

## Verification Already Run

These commands passed after the latest changes:

```bash
python3 -m py_compile scripts/route_inventory.py
python3 scripts/route_inventory.py --fail-on-unresolved
python3 -m json.tool reports/discord-parity.json >/dev/null
python3 -m json.tool reports/raw/route-inventory.json >/dev/null
```

The inventory run reported `169 routes, 0 unresolved` and produced these auth-class counts:

- `admin`: 7 (the `/api/v1/admin/*` surface)
- `user`: 144 (handlers that take an `AuthUser` extractor)
- `public`: 18 (auth flows, federation server-to-server endpoints, gateway upgrade, health, invite resolution, discover, user search, webhook execute-by-token)

## GitHub Issue Map

Primary source-of-truth issues:

- `#1` Discord parity master plan
- `#2` Milestone -1: brutal audit and claim demotion
- `#4` Messaging parity
- `#5` Roles and permissions parity
- `#7` Voice, video, and stage parity
- `#8` Apps, slash commands, bots, and webhooks parity

Issue `#5` should reference `reports/raw/route-inventory.json` as the authoritative list when triaging which routes still need permission gates. The inventory is generated, so it can be regenerated and diffed in any future session — do not hand-edit it.

## Current Next TODOs

Recommended order for the next agent:

1. Triage the 60 authenticated routes in `reports/raw/route-inventory.json` whose `permissions` array is empty and `flags` is empty — for each, decide whether `auth-only` is the correct posture (e.g. own-resource endpoints like `users/@me/...`) or whether a gate is missing. Update the relevant handler and re-run the generator.
2. Start the Playwright parity harness for messaging and roles.
3. Broaden permission UI proof to owner/admin/mod/member/muted/banned workflows.
4. Document Emma Bot credentials and test protocol without exposing secrets.
5. Create a two-client voice Playwright/manual checklist for join/leave/mute/deafen/screen-share.

## Exact Next Task Candidate

Best immediate task:

Run the route-inventory triage. Start with handlers whose path begins with `/api/v1/servers/`, `/api/v1/channels/`, or `/api/v1/messages/` and have neither permission gates nor a documented "own resource" pattern. Add the missing `require_server_perm` / `require_channel_perm` calls and add API smoke coverage in `scripts/permission_smoke.py` for each new gate.

## Caution

- Do not mark Discord parity features done just because tables, routes, or UI components exist.
- Do not expose tokens, admin credentials, or Emma Bot credentials in docs, GitHub comments, or reports.
- Keep `reports/discord-parity.json` aligned with GitHub issues when status changes.
- `reports/raw/route-inventory.json` is generated. Re-run `python3 scripts/route_inventory.py` after any router/handler change instead of editing it by hand.
- Use `cargo fmt --check` before finalizing Rust changes; the repo is now formatted.
