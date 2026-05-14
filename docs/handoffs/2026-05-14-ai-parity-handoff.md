# OpenCorde AI Handoff - 2026-05-14

This file is the first context a new AI agent should read before continuing the OpenCorde parity work.

## Repository State

- Repository: `King4s/OpenCorde.com`
- Local path: `/home/mb/opencorde`
- Branch: `main`
- Last pushed functional commit before this update: `d1aecc8 audit(routes): generate Axum route inventory with permission gates`
- Earlier relevant commits: `231d8e8` permissions UI proof, `ae6ed3b` LiveKit health proof, `5dadd96` LiveKit health backend.

At the time this handoff was updated, the next task was completed locally and should be committed/pushed with this file: tightened route-inventory recognizers (owner/author/member checks + path-shape classification) and a focused needs_review list.

## What Changed Most Recently

The route-inventory hardening pass completed after `d1aecc8`:

- `scripts/route_inventory.py` (schema bumped to v2) now also detects inline `owner_id == auth.user_id` and `author_id == auth.user_id` checks plus caller-`member_repo::get_member` lookups, and classifies each route's `path_kind` (own_resource, admin, auth, federation, mesh, infrastructure, generic).
- Own-resource paths now include `/@me`, `/me/`, `/api/v1/friends`, `/api/v1/push/`. Infrastructure includes `/api/v1/health`, `/api/v1/gateway`, `/api/v1/unfurl`.
- Inventory output gained a top-level `needs_review_routes` list and `summary.needs_review` count: 27 routes (down from 60 in the first pass) where auth=user, path_kind=generic, and no permission/admin/owner/author/member/verification gate was detected.
- `reports/discord-parity.json` updated to reflect the schema v2, the smaller needs_review list, and a per-surface gap breakdown so the next agent can pick a slice without rereading the inventory.

Functional files to inspect first:

- `scripts/route_inventory.py`
- `reports/raw/route-inventory.json` (regenerated)
- `reports/discord-parity.json` (roles_permissions area + next_session_focus)
- `crates/opencorde-api/src/routes/permission_check.rs` (the helpers the inventory looks for)

## Verification Already Run

These commands passed after the latest changes:

```bash
python3 -m py_compile scripts/route_inventory.py
python3 scripts/route_inventory.py --fail-on-unresolved
python3 -m json.tool reports/discord-parity.json >/dev/null
python3 -m json.tool reports/raw/route-inventory.json >/dev/null
```

Inventory totals: 169 endpoints, 0 unresolved handlers, by auth class admin=7 / user=144 / public=18, by path kind generic=120 / own_resource=19 / auth=12 / admin=7 / mesh=4 / federation=4 / infrastructure=3, needs_review=27.

## GitHub Issue Map

Primary source-of-truth issues:

- `#1` Discord parity master plan
- `#2` Milestone -1: brutal audit and claim demotion
- `#4` Messaging parity
- `#5` Roles and permissions parity
- `#7` Voice, video, and stage parity
- `#8` Apps, slash commands, bots, and webhooks parity

Issue `#5` should reference `reports/raw/route-inventory.json` (`needs_review_routes`) as the authoritative gate-triage list. Re-run the generator after every router/handler change; do not hand-edit the JSON.

## Current Next TODOs

Recommended order for the next agent:

1. Walk the `needs_review_routes` list in `reports/raw/route-inventory.json` and add gates. Suggested groupings:
   - **Server admin (likely MANAGE_SERVER)**: bridge mappings (POST/PATCH/DELETE/GET on `/api/v1/servers/{server_id}/bridge/mappings*`), automod read (`GET /api/v1/servers/{server_id}/automod`), discovery patch (`PATCH /api/v1/servers/{id}/discovery`).
   - **Emojis (MANAGE_GUILD_EXPRESSIONS)**: GET/POST `/api/v1/servers/{id}/emojis`, DELETE `/api/v1/servers/{id}/emojis/{emoji_id}`.
   - **Events**: server-membership for read+RSVP, MANAGE_EVENTS for create/edit/delete (5 routes under `/api/v1/events/{event_id}*`).
   - **DMs**: participant check on `GET/POST /api/v1/channels/@dms/{dm_id}/messages`.
   - **Members**: VIEW server perm on `GET /api/v1/servers/{server_id}/members`; MANAGE_NICKNAMES/MANAGE_ROLES on `PATCH /api/v1/servers/{server_id}/members/{user_id}`.
   - **Channel notification settings** (PUT/DELETE `/api/v1/channels/{id}/notification-settings`): VIEW_CHANNEL or server membership.
   - **Voice/stage** (`POST /api/v1/voice/leave`, `PATCH /api/v1/voice/state`, `DELETE /api/v1/channels/{channel_id}/stage/leave`): own-state but cross-checked against current LiveKit/voice presence.
   - **Ambiguous-by-design** (intentional auth-only): `GET /api/v1/servers` (caller's own server list), `POST /api/v1/servers` (anyone authed can create), `GET /api/v1/servers/{id}` (consider membership or document open-by-design), `GET /api/v1/users/{id}` (Discord allows for any authed user).
2. After each gate added, regenerate the inventory and confirm `needs_review` shrinks.
3. Add API smoke coverage for each new gate in `scripts/permission_smoke.py` (denial path).
4. Start the Playwright parity harness for messaging and roles.
5. Document Emma Bot credentials and test protocol without exposing secrets.
6. Two-client voice Playwright/manual checklist.

## Exact Next Task Candidate

Best immediate task:

Pick the bridge-mappings group (4 routes, all under `/api/v1/servers/{server_id}/bridge/mappings*`). Add `permission_check::require_server_perm(.., Permissions::MANAGE_SERVER)` at the top of each handler in `crates/opencorde-api/src/routes/bridge.rs`, add a denial smoke test in `scripts/permission_smoke.py`, regenerate the inventory, and verify `needs_review` drops by 4.

## Caution

- Do not mark Discord parity features done just because tables, routes, or UI components exist.
- Do not expose tokens, admin credentials, or Emma Bot credentials in docs, GitHub comments, or reports.
- Keep `reports/discord-parity.json` aligned with GitHub issues when status changes.
- `reports/raw/route-inventory.json` is generated. Re-run `python3 scripts/route_inventory.py` after any router/handler change instead of editing it by hand.
- The `needs_review` heuristic is conservative: a route may already be safe via a path/lookup pattern the inventory doesn't recognize. Always confirm by reading the handler before adding a gate.
- Use `cargo fmt --check` before finalizing Rust changes; the repo is now formatted.
