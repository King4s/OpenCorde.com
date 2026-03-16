# /crates/opencorde-db/

PostgreSQL database layer via sqlx. Library crate providing connection pooling and repository CRUD.

## Entry Point

- **src/lib.rs**: Pool creation, migration running, health checks

## Modules

| Module | Purpose |
|--------|---------|
| src/repos | Repository pattern: 5 repo modules (user, server, channel, message, member) |
| migrations | 9 SQL migration files (001_users through 009_files) |

## Key Functions in lib.rs

```rust
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error>
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError>
pub async fn health_check(pool: &PgPool) -> Result<(), sqlx::Error>
```

All repository functions follow:
```rust
#[tracing::instrument(skip(pool))]
pub async fn operation(pool: &PgPool, args...) -> Result<RowType, sqlx::Error>
```

## Migrations

9 SQL files (applied in order):
1. **001_users** - User accounts, auth, profiles
2. **002_servers** - Server entities, ownership
3. **003_channels** - Text/voice channels, hierarchy
4. **004_messages** - Message content, editing
5. **005_roles** - Server roles, permissions bitfield
6. **006_server_members** - Membership joins, role assignments
7. **007_invites** - Invite codes, expiry
8. **008_voice_states** - Active voice connections
9. **009_files** - File uploads, metadata

## Dependencies

- `sqlx` 0.8.3 - Query building, migrations, type-safe rows
- `chrono` - DateTime with timezone
- `tracing` - Structured logging
- `opencorde_core` - Snowflake ID type

## Testing

```bash
cargo test -p opencorde-db -- --lib
```

Requires DATABASE_URL env var for integration tests (not run without `--test`).
