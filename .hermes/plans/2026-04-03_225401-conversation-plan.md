# OpenCorde work-start plan

## Goal
Get into the OpenCorde project cleanly and be ready to execute the next requested task with minimal thrash.

## Current context / assumptions
- Active project is OpenCorde.
- Repository path previously identified: `/home/mb/opencorde`.
- The user asked to open the project because we will work on it, but has not yet specified the exact feature, bug, or area.
- This is a planning-only turn, so no implementation, repo edits, or mutating commands should be performed.

## What I inspected
- `ReadMeFirst.md`
- `tasks.md`
- `project-map.yaml`
- Presence of module `INDEX.md` files across backend, client, tests, deploy, and bridge areas

## High-level repo understanding
- Backend: Rust workspace with `opencorde-api`, `opencorde-db`, `opencorde-core`, `opencorde-crypto`, `opencorde-bridge`, `opencorde-search`
- Frontend: SvelteKit + Tauri in `client/`
- Key backend entrypoint: `crates/opencorde-api/src/main.rs`
- Key client entrypoint: `client/src/routes/+layout.svelte`
- Project status in docs says major planned phases are complete, so likely upcoming work will be bugfixes, follow-up polish, or new incremental features

## Proposed approach
Use a short discovery-first workflow before touching code:
1. Confirm the exact task from the user.
2. Narrow the affected subsystem.
3. Read the local `INDEX.md` files and relevant source files for that subsystem.
4. Reproduce or define the target behavior.
5. Implement with the smallest coherent change set.
6. Validate with targeted tests first, then broader checks if needed.

## Step-by-step plan

### Phase 1: Task definition
1. Get the exact work item from the user.
2. Classify it as one of:
   - backend API / WebSocket
   - database / migration
   - Svelte UI / store logic
   - Tauri desktop behavior
   - bridge integration
   - deployment / ops
   - test failure / regression
3. Capture acceptance criteria in one or two concrete sentences before editing code.

### Phase 2: Localize the code area
1. Open the nearest `INDEX.md` files for the relevant area.
2. Inspect the primary files for that path.
3. Trace related types/contracts across backend and frontend.
4. Identify exact files likely to change before implementation starts.

### Phase 3: Implementation planning for the chosen task
1. Determine whether the change needs:
   - API contract updates
   - DB migration
   - store updates
   - UI component updates
   - WebSocket event updates
   - tests / fixtures / browser coverage
2. Keep file size and project conventions in mind:
   - Rust files under 300 lines
   - Svelte/component files under 300 lines
   - use existing naming and module conventions
3. Prefer targeted changes over broad refactors unless the task explicitly requires restructuring.

### Phase 4: Validation strategy
1. Run the smallest relevant validation first, depending on the task:
   - Rust crate tests for backend changes
   - targeted `cargo test -p <crate>` where possible
   - frontend lint/type checks for Svelte/TS work
   - browser/E2E coverage only when the change affects user flows
2. If the task touches shared contracts, run both backend and frontend validation.
3. If behavior is real-time, verify both API response shape and WebSocket/store updates.

### Phase 5: Finish quality pass
1. Re-read changed files for convention compliance.
2. Check whether docs or `INDEX.md` files need updates.
3. Summarize what changed, why, and how it was validated.

## Files likely to matter soon
These are the most likely navigation targets once the task is specified.

### Repo-level
- `ReadMeFirst.md`
- `tasks.md`
- `project-map.yaml`
- `docker-compose.yml`
- `.env.example`

### Backend
- `crates/opencorde-api/src/main.rs`
- `crates/opencorde-api/src/lib.rs`
- `crates/opencorde-api/src/routes/`
- `crates/opencorde-api/src/ws/`
- `crates/opencorde-db/src/repos/`
- `crates/opencorde-core/src/models/`
- `crates/opencorde-crypto/src/`
- `crates/opencorde-bridge/src/`

### Frontend
- `client/src/routes/+layout.svelte`
- `client/src/routes/`
- `client/src/lib/api/`
- `client/src/lib/stores/`
- `client/src/lib/components/`
- `client/src/lib/livekit/`
- `client/src/lib/crypto/`

### Tests
- `tests/`
- `browser_test.py`

## Likely test / verification targets
Depending on the task:
- `cargo test --workspace`
- `cargo test -p opencorde-api`
- `cargo test -p opencorde-db`
- `cargo test -p opencorde-core`
- client lint/typecheck commands from the frontend workspace
- relevant browser/E2E tests in `tests/`

## Risks / tradeoffs
- The project is broad and cross-cutting; many features span backend + client + WebSocket.
- Docs say many phases are complete, so undocumented regressions or drift may exist between docs and code.
- Without a precise task, any deeper planning would become guesswork.
- Some changes may require coordination across Rust API types and TS client types.

## Open questions
1. What exact task should we work on first?
2. Is this a bugfix, new feature, refactor, test failure, or deployment issue?
3. If it is a bug, what is the reproduction path and expected behavior?
4. If it is a feature, which user flow should change?

## Recommended next move
Once the user names the task, immediately inspect the relevant subsystem `INDEX.md` plus the exact source files, then make a task-specific implementation plan or proceed directly if the scope is clear.