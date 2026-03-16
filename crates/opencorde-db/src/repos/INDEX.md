# /crates/opencorde-db/src/repos/

Repository pattern — one module per domain entity. Each provides async CRUD via PgPool.

## Files & Functions

| File | Entity | Key Functions |
|------|--------|-----------|
| user_repo.rs | Users | create, get_by_id, get_by_email, get_by_username, update_avatar, update_status |
| server_repo.rs | Servers | create, get_by_id, list_by_user, update, update_icon, delete |
| channel_repo.rs | Channels | create, get_by_id, list_by_server, update, update_position, delete |
| message_repo.rs | Messages | create, get_by_id, list_by_channel (cursor pagination), update_content, delete |
| member_repo.rs | Members | add, remove, get_member, list_by_server, update_nickname, add_role, remove_role, list_member_roles |
| mod.rs | Meta | Exports all repos |

## Function Signature Pattern

```rust
#[tracing::instrument(skip(pool))]
pub async fn operation(
    pool: &PgPool,
    id: Snowflake,
) -> Result<RowType, sqlx::Error>
```

All functions:
- Accept `&PgPool` reference (no connection management)
- Return `Result<RowType, sqlx::Error>`
- Use `#[tracing::instrument]` for structured logging
- Handle Snowflakes: convert with `.as_i64()` for binding, `Snowflake::new()` for conversion

## Row Types

Each repo defines `*Row` structs with `#[derive(sqlx::FromRow)]`:

```rust
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserRow {
    pub id: i64,                      // Snowflake stored as BIGINT → i64
    pub username: String,
    // ...
}
```

## Message Pagination

`list_by_channel()` supports cursor-based pagination:
- `before`: fetch messages with ID < cursor (newer first)
- `after`: fetch messages with ID > cursor (older first)
- `limit`: capped at 100 messages

## Testing

Unit tests in each module test row creation and basic logic:

```bash
cargo test -p opencorde-db -- --lib
```

Integration tests require a live PostgreSQL instance (not enabled without `--test` flag).
