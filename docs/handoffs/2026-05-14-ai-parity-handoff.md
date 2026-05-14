# OpenCorde AI Handoff - 2026-05-14

This file is the first context a new AI agent should read before continuing the OpenCorde parity work.

## Repository State

- Repository: `King4s/OpenCorde.com`
- Local path: `/home/mb/opencorde`
- Branch: `main`
- Last pushed functional commit before this update: `79f97db test(messaging): live UI proof for daily-chat actions + fix currentUser restore`
- Earlier relevant commits: `ef166ed` permission denial smoke, `52a9902` 7 permission gates, `3c7f70a` transitive recognizer resolution, `d1aecc8` initial route inventory.

At the time this handoff was updated, the next task was completed locally and should be committed/pushed with this file: attachment upload scenario added to the messaging UI harness.

## What Changed Most Recently

The attachment scenario completed after `79f97db`:

- `scripts/messaging_ui_qa.py` gains a seventh scenario: `_PNG_1x1` (70-byte inline RGBA PNG, IDAT-CRC valid) is uploaded through the hidden `<input type="file">` via `set_input_files`, then a message carrying the attachment is sent. The new message row is verified to contain an `<img>` element, screenshot saved as `07-attachment.png`.
- The cleanup pass now removes 2 messages per run (the original send/edit/pin'd message and the new attachment message — the reply gets deleted by the delete scenario).
- The PWA icon (`/app-icon-*`) was added to the harness's `_IGNORED_ABORT_PATTERNS` so a missing optimistic asset fetch doesn't poison `ok`.
- The initial-input wait was bumped from 20s → 45s to absorb hydration races on first navigation.
- `reports/raw/messaging-ui-proof.json` regenerated with 7 scenarios + 7 screenshots; `ok: true`, 0 failed requests, 2 fixture messages removed.
- `reports/discord-parity.json` messaging area observed list updated; the attachment gap closed; "attachment upload" removed from next_todos.

Verification:

- `python3 -m py_compile scripts/messaging_ui_qa.py` ✓.
- `python3 -m json.tool reports/discord-parity.json >/dev/null` ✓.
- `python3 -m json.tool reports/raw/messaging-ui-proof.json >/dev/null` ✓.
- `OC_MEMBER_EMAIL=browsertest@opencorde.com python3 scripts/messaging_ui_qa.py` ✓ — `ok: true` on 2 consecutive runs, 7 scenarios green.

## Operational Notes

- The `_PNG_1x1` constant is a 1x1 red RGBA PNG generated inline (`struct` + `zlib`). It is intentionally trivial so the script doesn't need a fixture file on disk. If you need a larger or differently-typed asset, generate it the same way.
- `set_input_files` works against the hidden `<input type="file">` element regardless of CSS visibility, but the message-input form still needs to be hydrated first. The initial-input `wait_for(timeout=45000)` covers slow first paints.
- Caddy serves `client/build/` directly; after any `pnpm build` the next browser load picks up the new bundle without restarting `opencorde-api.service`. The systemd service only carries Rust changes.

## GitHub Issue Map

Primary source-of-truth issues:

- `#1` Discord parity master plan
- `#2` Milestone -1: brutal audit and claim demotion
- `#4` Messaging parity — send/edit/reply/react/pin/delete/attachment all have first live proof. Remaining gaps: jump-to-message, grouping/date separators, two-client realtime, denied SEND_MESSAGES UX.
- `#5` Roles and permissions parity
- `#7` Voice, video, and stage parity
- `#8` Apps, slash commands, bots, and webhooks parity

## Current Next TODOs

Recommended order for the next agent:

1. Extend `scripts/messaging_ui_qa.py`:
   - **Denied SEND_MESSAGES**: log in as a limited fixture user with a channel override that removes `SEND_MESSAGES`, try to send, assert the POST returns 403 or the input is disabled. Reuse the limited fixture from `permissions_ui_qa.py`.
   - **Jump-to-message**: click the reply-context bubble of the existing reply scenario, assert the page scrolls and `#msg-{id}` becomes visible.
   - **Two-client realtime**: spawn a second Playwright context with a different fixture user (already in the same server), open the same channel, then have the first context send a message and assert the second context's `#msg-{id}` row appears without a refresh.
2. Document Emma Bot credentials and test protocol (Issue `#8`) without exposing secrets.
3. Two-client voice Playwright/manual checklist (Issue `#7`).
4. Add owner/admin/mod/member/muted/banned permission matrix in `scripts/permission_smoke.py`.

## Exact Next Task Candidate

Best immediate task:

Add the **denied SEND_MESSAGES** scenario to `prove_messaging`. Use the same limited fixture user `permissions_ui_qa.py` builds (or build a thin wrapper that gives a limited token), and a private channel where SEND_MESSAGES is denied for `@everyone`. The expected behavior: typing a message and pressing Enter does not produce a new row in the message list; the POST returns 403 visible in `failedRequests`. Screenshot as `08-send-denied.png`. Update `report["ok"]` to include the new check.

## Caution

- Do not mark Discord parity features done just because tables, routes, or UI components exist.
- Do not expose tokens, admin credentials, or Emma Bot credentials in docs, GitHub comments, or reports.
- Keep `reports/discord-parity.json` aligned with GitHub issues when status changes.
- `reports/raw/route-inventory.json` is generated. Re-run `python3 scripts/route_inventory.py` after any router/handler change instead of editing it by hand.
- The new `restoreSession()` call in `routes/servers/+layout.svelte` runs on every full reload of an authed page. If that surface ever needs to skip auth restore (e.g. share-link landing pages mounted under `/servers/`), gate it explicitly.
- The harness's `_IGNORED_ABORT_PATTERNS` filters chromium-cleanup `net::ERR_ABORTED` noise. If a real regression triggers an aborted request matching one of those patterns, you will not catch it from `failedRequests`; check `ignoredFailedRequests` as well during investigations.
- Use `cargo fmt --check` before finalizing Rust changes; the repo is now formatted.
