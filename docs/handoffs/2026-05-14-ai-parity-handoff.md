# OpenCorde AI Handoff - 2026-05-14

This file is the first context a new AI agent should read before continuing the OpenCorde parity work.

## Repository State

- Repository: `King4s/OpenCorde.com`
- Local path: `/home/mb/opencorde`
- Branch: `main`
- Last pushed functional commit before this update: `ef166ed test(permissions): live API denial smoke for 8 new gates`
- Earlier relevant commits: `52a9902` 7 permission gates, `3c7f70a` transitive recognizer resolution, `a4bff96` recognizer hardening, `d1aecc8` initial route inventory.

At the time this handoff was updated, the next task was completed locally and should be committed/pushed with this file: live Playwright UI proof for the daily messaging workflows (Issue #4), plus a real app bug fix surfaced while building the harness.

## What Changed Most Recently

The messaging-parity proof pass completed after `ef166ed`:

- Added `scripts/messaging_ui_qa.py` — Playwright harness that drives the chat UI as the fixture owner and proves: send, edit (with "edited" indicator), reply (with reply-context bubble), react (badge visible), pin (visible in pinned panel), delete (row removed). Output: `reports/raw/messaging-ui-proof.json`, screenshots `01-send.png … 06-delete.png` under `reports/parity-screenshots/messaging-ui/`. Cleans up its own messages at the end.
- Fixed an app bug in `client/src/routes/servers/+layout.svelte`: after a full page reload, the in-memory `$currentUser` was never restored from the persisted token, so `isOwn` was false and edit/delete/pin context-menu buttons stayed hidden. The layout now calls `restoreSession()` when it sees a stored token. This is a real product fix — every refresh of `/servers/...` previously dropped the owner-only UI affordances.
- Rebuilt `client/build` (`pnpm build`) so Caddy serves the patched layout. The release-mode `opencorde-api` binary is unchanged from `ef166ed`.
- `reports/discord-parity.json` messaging area moved from `shallow_needs_proof` to `partial`, with the new observed proof + the app fix called out. `evidence_sources` registers the new artifact.

Verification:

- `pnpm check` ✓ — 474 files, 0 errors, 0 warnings.
- `pnpm build` ✓ — fresh `client/build/`.
- `python3 -m py_compile scripts/messaging_ui_qa.py` ✓.
- `python3 -m json.tool reports/discord-parity.json >/dev/null` ✓.
- `python3 -m json.tool reports/raw/messaging-ui-proof.json >/dev/null` ✓.
- `OC_MEMBER_EMAIL=browsertest@opencorde.com python3 scripts/messaging_ui_qa.py` ✓ — `ok: true`, all 6 scenarios pass, 1 cleanup message removed, 6 screenshots saved.

## Operational Notes

- `scripts/messaging_ui_qa.py` runs against `https://opencorde.com` by default. Override the test fixture email via `OC_MEMBER_EMAIL` and the password via `OC_MEMBER_PASSWORD` — never commit fixture credentials into docs or reports.
- The script auto-accepts any `window.confirm()` dialogs that pop up during the action sequence. None are currently triggered by the chat UI but stage cleanup or future confirmations would not break the harness.
- A small set of `net::ERR_ABORTED` failures (SvelteKit prefetch chunks, `/ack` POSTs, pin PUT, delete DELETE) are ignored: the corresponding HTTP responses are received before chromium reports the abort on context teardown. They land in `browser.browser.ignoredFailedRequests` for visibility.
- If the chat UI is ever changed to require a different message-row anchor than `#msg-{id}`, update `message_row()` in the harness.

## GitHub Issue Map

Primary source-of-truth issues:

- `#1` Discord parity master plan
- `#2` Milestone -1: brutal audit and claim demotion
- `#4` Messaging parity — daily-chat actions now have first live proof; remaining gaps: attachment upload, jump-to-message, grouping/date separators, two-client realtime, denied SEND_MESSAGES UX.
- `#5` Roles and permissions parity
- `#7` Voice, video, and stage parity
- `#8` Apps, slash commands, bots, and webhooks parity

## Current Next TODOs

Recommended order for the next agent:

1. Extend `scripts/messaging_ui_qa.py`:
   - Attachment upload + display (drag/drop or file input on the message form).
   - Jump-to-message via the reply-context bubble (`scrollToMessage`).
   - Denied SEND_MESSAGES: log in as a limited fixture user with no `SEND_MESSAGES` override, assert the input is disabled or the POST fails.
2. Add a two-client realtime check: one Playwright context sends, another (same channel, different fixture user) observes the WS-driven update without a refresh.
3. Document Emma Bot credentials and test protocol (Issue `#8`) without exposing secrets.
4. Two-client voice Playwright/manual checklist (Issue `#7`).
5. Add owner/admin/mod/member/muted/banned permission matrix in `scripts/permission_smoke.py`.

## Exact Next Task Candidate

Best immediate task:

Extend `prove_messaging` with an attachment upload step. The chat input has an `aria-label="Attach file"` button that triggers a hidden file input. Use `page.set_input_files()` on the underlying `input[type="file"]` (find via the form), send a small fixture image, assert the new message row contains an `<img>` with the uploaded URL, screenshot it as `07-attachment.png`.

## Caution

- Do not mark Discord parity features done just because tables, routes, or UI components exist.
- Do not expose tokens, admin credentials, or Emma Bot credentials in docs, GitHub comments, or reports.
- Keep `reports/discord-parity.json` aligned with GitHub issues when status changes.
- `reports/raw/route-inventory.json` is generated. Re-run `python3 scripts/route_inventory.py` after any router/handler change instead of editing it by hand.
- The new `restoreSession()` call in `routes/servers/+layout.svelte` runs on every full reload of an authed page. If that surface ever needs to skip auth restore (e.g. share-link landing pages mounted under `/servers/`), gate it explicitly.
- Use `cargo fmt --check` before finalizing Rust changes; the repo is now formatted.
