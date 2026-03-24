# Unread Message Tracking Implementation

## Overview
Implemented unread message tracking backend for OpenCorde using Rust/Axum. Tracks the last-read message per user per channel and provides APIs to mark channels as read and retrieve read states.

## Files Created

### 1. Repository Layer
**File**: `F:/AI-Projekter/DA/commune/crates/opencorde-db/src/repos/read_state_repo.rs`
**Lines**: 152 (under 300 limit)

#### Components:
- **ReadStateRow** struct: Maps database rows with fields:
  - `user_id: i64`
  - `channel_id: i64`
  - `last_read_id: i64`
  - `mention_count: i32`
  - `updated_at: DateTime<Utc>`

- **mark_read()** function:
  - Upsert operation on `channel_read_state` table
  - Uses `GREATEST()` to only move read position forward (monotonic)
  - Resets `mention_count` to 0 on each mark as read
  - Updates `updated_at` timestamp

- **get_for_user()** function:
  - Fetches all read states for a given user
  - Orders by `updated_at DESC` (most recently updated first)
  - Returns `Vec<ReadStateRow>`

- **count_unread()** function:
  - Counts messages in a channel newer than `last_read_id`
  - Uses `COALESCE()` to default to 0 if no read state exists
  - Efficient single-query approach

#### Error Handling:
- All functions return `Result<T, sqlx::Error>`
- Errors propagated to routes with `map_err(ApiError::Database)`

#### Logging:
- Structured logging with `#[tracing::instrument(skip(pool))]`
- All operations log user_id, channel_id, and operation details

---

### 2. Routes Layer
**File**: `F:/AI-Projekter/DA/commune/crates/opencorde-api/src/routes/read_state.rs`
**Lines**: 150 (under 300 limit)

#### Request Types:
- **AckRequest**
  ```rust
  pub struct AckRequest {
      pub message_id: String,  // Last message ID read
  }
  ```

#### Response Types:
- **ReadStateResponse**
  ```rust
  pub struct ReadStateResponse {
      pub channel_id: String,
      pub last_read_id: String,
      pub mention_count: i32,
  }
  ```

#### Endpoints:

##### POST /api/v1/channels/{channel_id}/ack
- **Purpose**: Mark a channel as read up to a specific message
- **Authentication**: Required (AuthUser middleware)
- **Request Body**: `AckRequest { message_id: String }`
- **Response**: `204 No Content`
- **Side Effects**:
  - Calls `read_state_repo::mark_read()` to update database
  - Broadcasts `ChannelAck` WebSocket event to other sessions of the same user
  - Event structure: `{ "type": "ChannelAck", "data": { "user_id", "channel_id", "last_read_id" } }`
- **Error Handling**:
  - `BadRequest` if channel_id is invalid Snowflake
  - `BadRequest` if message_id cannot parse as i64
  - `Database` error if operation fails

##### GET /api/v1/users/@me/read-states
- **Purpose**: Get all read states for the authenticated user
- **Authentication**: Required (AuthUser middleware)
- **Response**: `Vec<ReadStateResponse>`
- **Behavior**:
  - Fetches all channels user has interacted with
  - Includes `channel_id`, `last_read_id`, and `mention_count`
  - Ordered by most recently updated first
- **Error Handling**:
  - `Database` error if query fails

#### Features:
- Comprehensive doc comments on all public items
- Structured logging with `#[instrument]` macro
- Snowflake ID parsing using `helpers::parse_snowflake()`
- Message ID validation (must be parseable as i64)
- WebSocket integration for real-time updates
- Proper error propagation with ApiError types

---

## Database Schema
**Migration**: `012_read_state.sql`

```sql
CREATE TABLE channel_read_state (
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id      BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    last_read_id    BIGINT NOT NULL DEFAULT 0,
    mention_count   INT NOT NULL DEFAULT 0,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, channel_id)
);

CREATE INDEX idx_read_state_user ON channel_read_state (user_id);
```

---

## Integration Points

### Existing Integrations (Pre-existing):
- `crates/opencorde-db/src/repos/mod.rs`: Already exports `pub mod read_state_repo;`
- `crates/opencorde-api/src/routes/mod.rs`: Already has:
  - `pub mod read_state;`
  - `.merge(read_state::router())` in `api_router()`

### Router Integration:
The `router()` function creates two routes:
```rust
Router::new()
    .route("/api/v1/channels/:channel_id/ack", post(mark_channel_read))
    .route("/api/v1/users/@me/read-states", get(get_user_read_states))
```

### WebSocket Integration:
Uses `state.event_tx` to broadcast events:
```rust
let event = serde_json::json!({
    "type": "ChannelAck",
    "data": {
        "user_id": auth.user_id.to_string(),
        "channel_id": channel_id_sf.to_string(),
        "last_read_id": message_id.to_string(),
    }
});

if state.event_tx.send(event).is_err() {
    tracing::debug!("no WebSocket subscribers for ChannelAck event");
}
```

---

## Testing

### Unit Tests Included:

**read_state_repo.rs**:
- `test_read_state_row_creation()`: Verifies ReadStateRow struct initialization
- `test_last_read_id_default_zero()`: Confirms default read position is 0

**read_state.rs**:
- `test_ack_request_deserialization()`: Validates JSON deserialization
- `test_read_state_response_serialization()`: Validates JSON serialization

### Manual Testing Recommendations:
1. Create a user and channel
2. Send messages to the channel
3. Call `POST /api/v1/channels/{channel_id}/ack` with a message_id
4. Verify database updated with correct `last_read_id`
5. Call `GET /api/v1/users/@me/read-states`
6. Verify response includes all previously acked channels
7. Monitor WebSocket for `ChannelAck` events on ack calls

---

## Code Quality

✓ Follows project patterns (async/await, error handling, logging)
✓ Proper use of Snowflake IDs for type safety
✓ No file exceeds 300 line limit
✓ Structured documentation with dependency comments
✓ Comprehensive error handling
✓ Non-blocking error logging
✓ Proper use of sqlx query builder with parameterized queries
✓ RESTful API design with appropriate HTTP methods and status codes
✓ WebSocket event broadcasting for real-time updates

---

## Dependencies
- `opencorde_core::snowflake::Snowflake`
- `sqlx`: Async SQL toolkit with PostgreSQL driver
- `chrono`: Date/time handling
- `axum`: Web framework
- `serde`: JSON serialization/deserialization
- `tracing`: Structured logging
