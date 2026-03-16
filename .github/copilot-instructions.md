# OpenCorde — GitHub Copilot Instructions

**ALWAYS read `ReadMeFirst.md` first.** It contains the full project map, architecture, and current state.

## Quick Reference

- **Project**: Self-hosted Discord alternative (Rust/Axum backend, SvelteKit/Tauri frontend)
- **Entry point**: `ReadMeFirst.md`
- **Architecture**: `project-map.yaml`
- **Decisions**: `decisions.md`
- **Tasks**: `tasks.md`
- **Quality gate**: `quality-gate.md`

## Rules (Non-Negotiable)

1. **No file over 300 lines** — split by responsibility at ~250 lines
2. **Structured file headers** — every source file starts with machine-parseable doc comment
3. **Update INDEX.md** — after creating/modifying files in any directory
4. **Update ReadMeFirst.md** — after any structural change
5. **Update project-map.yaml** — after adding new crates/modules
6. **Update tasks.md** — check off completed tasks
7. **Update decisions.md** — log any architectural choice with reasoning
8. **Dependency comments** — every dep in Cargo.toml/package.json has a WHY comment
9. **Never guess** — research APIs, verify library versions, check current docs
10. **Test before done** — compile, clippy, test, manual verify. Fix failures silently.

## Conventions

- Rust files: `snake_case.rs`, one primary struct/trait per file
- Svelte components: `PascalCase.svelte`
- Stores: `camelCase.ts`
- Routes: `kebab-case/` directories
- DB migrations: `NNN_descriptive_name.sql`
- Error handling: `thiserror` for library errors, `anyhow` for application errors
- Async runtime: Tokio
- Logging: `tracing` crate, structured fields, request ID on every request
- Git: Conventional Commits — `feat(scope): description`

## Logging Standard

```rust
#[tracing::instrument(skip(pool))]
async fn example(pool: &PgPool) -> Result<()> {
    tracing::info!("starting operation");
    // ...
    tracing::info!(entity_id = %id, "operation completed");
    Ok(())
}
```

Levels: ERROR (failures), WARN (degraded), INFO (operations), DEBUG (detail), TRACE (verbose)
