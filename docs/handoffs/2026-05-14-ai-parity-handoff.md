# OpenCorde AI Handoff - 2026-05-14

This file is the first context a new AI agent should read before continuing the OpenCorde parity work.

## Repository State

- Repository: `King4s/OpenCorde.com`
- Local path: `/home/mb/opencorde`
- Branch: `main`
- Last pushed functional commit before this update: `a4bff96 audit(routes): tighten inventory recognizers, surface needs_review list`
- Earlier relevant commits: `d1aecc8` initial route inventory, `231d8e8` permissions UI proof, `ae6ed3b` LiveKit health proof.

At the time this handoff was updated, the next task was completed locally and should be committed/pushed with this file: transitive helper resolution in the route inventory generator. The needs_review list narrowed to 14 candidates with 7 likely-real gaps and 7 ambiguous-by-design.

## What Changed Most Recently

The inventory recognizer pass completed after `a4bff96`:

- `scripts/route_inventory.py` now recognizes named-helper gates (`check_server_owner`, `is_dm_member`) and resolves wrapper helpers transitively. Scope: same file + same directory. The handler's own fn name is excluded so a signature can't lift gates onto itself. Same-dir scope is what makes the `events/rsvp.rs` → `events/handlers.rs::require_event_visible` pattern light up without name collisions bleeding across unrelated modules like `admin/`.
- File-scoped resolution fixed a false positive: previously the user `list_servers` in `servers/handlers/crud.rs` inherited the admin flag from the admin `list_servers` in `admin/handlers.rs` because of a name collision.
- `needs_review` dropped from 27 to 14. Of those 14, 7 are likely real gate gaps and 7 are intentionally auth-only by Discord-style design (caller-implicit own state or open fetch).

Functional files to inspect first:

- `scripts/route_inventory.py` (the `transitive_annotations` + `visible_helpers` functions)
- `reports/raw/route-inventory.json` (regenerated; `needs_review_routes` is the triage list)
- `reports/discord-parity.json` (roles_permissions area enumerates the 7+7 split)

## Verification Already Run

These commands passed after the latest changes:

```bash
python3 -m py_compile scripts/route_inventory.py
python3 scripts/route_inventory.py --fail-on-unresolved
python3 -m json.tool reports/discord-parity.json >/dev/null
python3 -m json.tool reports/raw/route-inventory.json >/dev/null
```

Inventory totals: 169 endpoints, 0 unresolved handlers, by auth class admin=7 / user=144 / public=18, by path kind generic=120 / own_resource=19 / auth=12 / admin=7 / mesh=4 / federation=4 / infrastructure=3, needs_review=14.

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

1. **Fix the 7 likely-real gaps**, one route at a time, regenerating the inventory after each:
   - `GET /api/v1/servers/{server_id}/automod` — handler `list_rules` in `routes/automod/handlers.rs` has a comment "no permission check needed for viewing rules". This is a misjudgment: rule keyword lists are sensitive. Add `permission_check::require_server_perm(.., Permissions::MANAGE_SERVER)` to match the other automod handlers.
   - `GET /api/v1/servers/{id}` — `get_server` in `routes/servers/handlers/crud.rs`. Add `permission_check::require_server_perm(.., Permissions::VIEW_CHANNEL)` (or equivalent membership check).
   - `GET /api/v1/servers/{id}/emojis` — `routes/emojis.rs`. Same membership gate.
   - `GET /api/v1/servers/{server_id}/members` — `routes/members.rs`. Membership gate.
   - `PATCH /api/v1/servers/{server_id}/members/{user_id}` — `routes/members.rs`. Gate on `MANAGE_NICKNAMES` (self-rename) or `MANAGE_ROLES` (other-rename). Check `require_member_below_actor` is in place when target is not self.
   - `PATCH /api/v1/servers/{id}/discovery` — `routes/discovery.rs`. Add `MANAGE_SERVER`.
   - `PUT /api/v1/channels/{id}/notification-settings` and `DELETE /api/v1/channels/{id}/notification-settings` — `routes/notification_settings.rs`. Decide: per-user per-channel preferences are caller-implicit; the gate should be "user must be able to see this channel" → `VIEW_CHANNEL`.
2. For each gate added, add an API denial smoke test in `scripts/permission_smoke.py`.
3. After all 7 are gated, the only `needs_review_routes` left should be the 7 ambiguous-by-design entries: `GET /api/v1/servers`, `POST /api/v1/servers`, `GET /api/v1/users/{id}`, `POST /api/v1/voice/leave`, `PATCH /api/v1/voice/state`, `DELETE /api/v1/channels/{channel_id}/stage/leave`. Either accept those as final and document them, or push back to the inventory by widening `path_kind`.
4. Start the Playwright parity harness for messaging and roles.
5. Document Emma Bot credentials and test protocol without exposing secrets.
6. Two-client voice Playwright/manual checklist.

## Exact Next Task Candidate

Best immediate task:

Gate `list_rules` in `crates/opencorde-api/src/routes/automod/handlers.rs` with `permission_check::require_server_perm(.., Permissions::MANAGE_SERVER)` to match the other automod handlers; remove the misleading "no permission check needed" comment. Add a denial smoke in `scripts/permission_smoke.py`. Regenerate inventory; confirm `needs_review` drops by 1.

## Caution

- Do not mark Discord parity features done just because tables, routes, or UI components exist.
- Do not expose tokens, admin credentials, or Emma Bot credentials in docs, GitHub comments, or reports.
- Keep `reports/discord-parity.json` aligned with GitHub issues when status changes.
- `reports/raw/route-inventory.json` is generated. Re-run `python3 scripts/route_inventory.py` after any router/handler change instead of editing it by hand.
- The `needs_review` heuristic is conservative: a route may already be safe via a path/lookup pattern the inventory doesn't recognize. Always confirm by reading the handler before adding a gate.
- Use `cargo fmt --check` before finalizing Rust changes; the repo is now formatted.
