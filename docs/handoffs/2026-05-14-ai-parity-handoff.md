# OpenCorde AI Handoff - 2026-05-14

This file is the first context a new AI agent should read before continuing the OpenCorde parity work.

## Repository State

- Repository: `King4s/OpenCorde.com`
- Local path: `/home/mb/opencorde`
- Branch: `main`
- Last functional commit before this handoff: `5dadd96 feat(admin): surface LiveKit health in instance stats`
- Previous relevant commit: `f9c6a76 fix(voice): speaking indicator, race condition guard, and leave 204`

At the time this handoff was started, `main` was pushed to `origin/main` and the working tree was clean. This handoff file and the corresponding `reports/discord-parity.json` pointer are intended to be committed immediately after the GitHub issue comments are posted.

## What Changed Most Recently

Commit `5dadd96` did two things:

- Added LiveKit operational health to `GET /api/v1/admin/stats`.
- Applied `cargo fmt` across the Rust workspace so `cargo fmt --check` passes.

Functional files to inspect first:

- `crates/opencorde-api/src/routes/admin/handlers.rs`
- `crates/opencorde-api/src/routes/admin/types.rs`
- `client/src/lib/api/types.ts`
- `client/src/routes/admin/+page.svelte`
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
```

## GitHub Issue Map

Primary source-of-truth issues:

- `#1` Discord parity master plan
- `#2` Milestone -1: brutal audit and claim demotion
- `#4` Messaging parity
- `#5` Roles and permissions parity
- `#7` Voice, video, and stage parity
- `#8` Apps, slash commands, bots, and webhooks parity

Issue `#7` should no longer be treated as needing the implementation of "LiveKit health to instance report"; that code is now in `5dadd96`. What remains is proof that the admin UI exposes it correctly.

## Current Next TODOs

Recommended order for the next agent:

1. Add Playwright or manual browser proof that `/admin` surfaces LiveKit local and public proxy health correctly.
2. Add Playwright UI proof for private-channel and role-management workflows.
3. Add route inventory JSON generated from Axum route declarations and permission annotations.
4. Start the Playwright parity harness for messaging and roles.
5. Document Emma Bot credentials and test protocol without exposing secrets.

## Exact Next Task Candidate

Best immediate task:

Create a small admin dashboard verification script or Playwright test that:

- logs in as an admin test user, or uses an existing authenticated admin browser state if available,
- opens `/admin`,
- confirms a `LiveKit Health` panel is visible,
- confirms both `Local` and `Public Proxy` rows are rendered,
- records the result under `reports/raw/` or links a screenshot/trace,
- updates `reports/discord-parity.json` and issue `#7` with the proof.

If admin credentials are not available locally, do not invent them. Fall back to an API-level proof for `/api/v1/admin/stats` only, and clearly mark UI proof as still blocked.

## Caution

- Do not mark Discord parity features done just because tables, routes, or UI components exist.
- Do not expose tokens, admin credentials, or Emma Bot credentials in docs, GitHub comments, or reports.
- Keep `reports/discord-parity.json` aligned with GitHub issues when status changes.
- Use `cargo fmt --check` before finalizing Rust changes; the repo is now formatted.
