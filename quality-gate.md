# Quality Gate Checklist

**Every task MUST pass ALL of these criteria before being marked complete.**

Use this checklist for self-review before committing code. CI/CD will enforce these automatically.

---

## Code Quality

### Rust

- [ ] Code compiles: `cargo check --workspace`
- [ ] No clippy warnings: `cargo clippy --workspace -- -D warnings`
- [ ] Formatted: `cargo fmt --check`
- [ ] All tests pass: `cargo test --workspace`
- [ ] No unsafe code blocks without `// SAFETY: ...` comment and documented invariants
- [ ] Error types implement `impl Display` for user-facing messages
- [ ] Logging uses `tracing` crate with structured fields (not `println!`)
- [ ] No hardcoded secrets (.env variables only)

### TypeScript / SvelteKit

- [ ] Lints: `pnpm lint` (ESLint + Prettier)
- [ ] All tests pass: `pnpm test`
- [ ] No TypeScript errors: `tsc --noEmit`
- [ ] Component props are documented with JSDoc comments
- [ ] Store subscriptions are properly unsubscribed (avoid memory leaks)
- [ ] No console.log in production code (use browser DevTools or logging service)

### SQL

- [ ] All SQL in migrations; no hardcoded queries in app code
- [ ] Migrations are idempotent (can run multiple times safely)
- [ ] Foreign key constraints enforce referential integrity
- [ ] Indexes created for high-cardinality columns (user_id, channel_id, created_at)
- [ ] Migrations tested: `sqlx migrate run`

---

## AI-Friendly Structure

- [ ] No file exceeds 300 lines (count logical lines, excluding blank lines and comments at EOF)
  - Large modules are split into submodules with INDEX.md
- [ ] New files have structured header comments:
  - Rust: `//! Module purpose. One sentence.`
  - TypeScript: `/** Module purpose. One sentence. */`
  - SQL: `-- Migration: purpose. Timestamp.`
- [ ] Relevant INDEX.md files created/updated:
  - Each crate: `src/INDEX.md`
  - Each module with submodules: `src/module/INDEX.md`
  - Lists files with 1-line descriptions + line counts
- [ ] ReadMeFirst.md updated if project structure changed
  - "Project map" section reflects new directories
  - "File index" updated with new modules
- [ ] project-map.yaml updated if:
  - New crate added
  - New dependencies added to Cargo.toml
  - New database tables added (update database section)
  - Client routes added

---

## Project Tracking

- [ ] tasks.md updated
  - Completed task checked off with [x]
  - Any new blockers noted
  - Next week's tasks are unblocked
- [ ] decisions.md updated (if new architectural choice made)
  - Decision has status (PROPOSED, ACCEPTED, REJECTED)
  - Rationale explains the "why"
  - Alternatives considered are listed
- [ ] Dependencies in Cargo.toml / package.json have justification comments
  - Example: `# SQLx: compile-time SQL checking`

---

## Testing

### Unit Tests

- [ ] New logic has unit tests in `#[cfg(test)]` modules
- [ ] Test coverage ≥70% for critical paths (auth, permissions, message creation)
- [ ] Test names describe behavior: `#[test] fn create_user_with_invalid_email_returns_error()`
- [ ] Mock external dependencies (database, Redis) in tests

### Integration Tests

- [ ] New endpoints have integration tests in `tests/` directory
  - Rust: `opencorde-api/tests/auth_integration.rs`
  - TypeScript: tests in `src/routes/+page.test.ts`
- [ ] End-to-end flows tested (register → login → create server → send message)
- [ ] Error cases tested (invalid input, unauthorized, not found, rate limit)
- [ ] WebSocket tests include connection lifecycle, heartbeat, event broadcasting

### Manual Testing

- [ ] If service/UI change: manual verification
  - REST endpoints: test with curl or Postman
  - WebSocket: verify connection, event delivery
  - UI: test on Windows, macOS, Linux (if desktop change)
- [ ] Logs checked for errors/warnings during manual test

### Performance

- [ ] No obvious N+1 queries
  - Use `.eager()` in sqlx or explicit JOINs
  - Test with EXPLAIN ANALYZE on slow queries
- [ ] WebSocket broadcast latency <100ms
  - Use `tracing::info!` to measure spans
- [ ] Desktop client startup time <3 seconds

---

## Security

- [ ] All user inputs validated before use
  - Length, type, format (email, URL, alphanumeric)
  - Return 400 Bad Request with specific error message
- [ ] SQL injection prevention: use parameterized queries (sqlx is safe by default)
- [ ] Authentication enforced on all protected endpoints (middleware checks)
- [ ] Rate limiting applied to auth endpoints
- [ ] Passwords hashed with Argon2id (never stored plaintext)
- [ ] JWTs validated on every request (check signature, expiry, claims)
- [ ] CORS configured: only allow safe origins (never `*` in production)
- [ ] Secrets (.env) never committed; .env.example has placeholder values only
- [ ] File uploads validated: check MIME type, size limit, scan for malware (future)
- [ ] Permission checks occur in handlers before database queries

---

## Documentation

- [ ] Public functions have doc comments (Rust: `///`, TypeScript: `/**`)
  - Describes purpose, parameters, return value, errors
  - Example: `/// Creates a new message. Publishes MESSAGE_CREATE event.`
- [ ] Complex logic has inline comments explaining "why", not "what"
  - Bad: `// Increment count` (what the code does)
  - Good: `// Increment uses to track consumed invites` (why)
- [ ] API endpoints documented in code or docs/api_reference.md
  - Method, path, auth, request/response examples
- [ ] Database schema documented in migrations or docs/schema.md
- [ ] README has setup instructions (future: docs/development.md)

---

## Deployment Readiness

- [ ] Environment variables configured in .env.example
- [ ] Docker image builds successfully (if applicable)
- [ ] Binary works on target platforms (Windows, macOS, Linux)
- [ ] Database migrations run without errors
- [ ] No hardcoded development IPs (use environment variables)
- [ ] Graceful shutdown implemented (cleanup connections, close WebSockets)

---

## Commit Hygiene

- [ ] Commit message follows format: `type(scope): description`
  - Examples: `feat(auth): add JWT refresh token rotation`, `fix(ws): handle reconnection race condition`
- [ ] Commit is atomic: one feature or fix per commit
- [ ] No debug code (console.log, dbg!, println!) in commit
- [ ] No merge conflicts left unresolved
- [ ] Branch rebased on main before PR (if applicable)

---

## Pre-Submission Checklist

Before marking a task complete:

1. [ ] Run `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --check && cargo test --workspace`
2. [ ] Run `pnpm lint && pnpm test` (if TypeScript changes)
3. [ ] Run `docker compose up -d && cargo run` (or `pnpm dev`)
4. [ ] Manually test the feature (steps in task.md)
5. [ ] Update relevant .md files (ReadMeFirst, project-map, decisions, tasks)
6. [ ] Commit with proper message format
7. [ ] Push to branch (if not main)

---

## Failure Recovery

If a quality gate check fails:

1. **Compilation error:** Fix the code, don't commit until `cargo check` passes.
2. **Clippy warning:** Address the warning or add `#[allow(clippy::...)]` with justification.
3. **Test failure:** Debug the test, fix the code, re-run tests.
4. **Formatting:** Run `cargo fmt` to auto-fix.
5. **Missing documentation:** Add doc comments or update .md files.
6. **Security issue:** Review the vulnerability, patch immediately, notify maintainers.

---

## Continuous Integration (Future)

When CI/CD is set up, these checks will run automatically on every push:

- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo fmt --check`
- `cargo test --workspace`
- `pnpm lint`
- `pnpm test`
- `docker compose up -d && cargo test --all-features --workspace`
- Code coverage reports

Merge to main only when CI passes.

---

**Last updated:** 2026-03-16
