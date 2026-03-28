//! # Repository: Audit Log
//! Write and read audit log entries for server moderation history.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake
//! - serde_json for changes tracking

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use serde_json::Value as JsonValue;
use sqlx::PgPool;

/// Row type for reading audit log entries from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuditLogRow {
    pub id: i64,
    pub server_id: i64,
    pub actor_id: Option<i64>,
    pub actor_username: Option<String>,
    pub action: String,
    pub target_id: Option<i64>,
    pub target_type: Option<String>,
    pub changes: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
}

/// Create an audit log entry.
///
/// Logs a moderation action (ban, kick, timeout, etc.) with optional before/after changes.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Snowflake ID for the audit log entry
/// * `server_id` - Snowflake ID of the server
/// * `actor_id` - Snowflake ID of the user performing the action
/// * `actor_username` - Username of the actor (denormalized)
/// * `action` - Action type (e.g., "member.ban", "member.kick", "member.timeout")
/// * `target_id` - Optional ID of the affected entity
/// * `target_type` - Optional type of the affected entity
/// * `changes` - Optional JSON object with before/after state
///
/// # Errors
/// Returns sqlx::Error if the insert fails.
#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip(pool))]
pub async fn log_action(
    pool: &PgPool,
    id: Snowflake,
    server_id: Snowflake,
    actor_id: Snowflake,
    actor_username: &str,
    action: &str,
    target_id: Option<i64>,
    target_type: Option<&str>,
    changes: Option<JsonValue>,
) -> Result<AuditLogRow, sqlx::Error> {
    tracing::info!(
        audit_id = id.as_i64(),
        server_id = server_id.as_i64(),
        actor_id = actor_id.as_i64(),
        action = %action,
        target_id = ?target_id,
        target_type = ?target_type,
        "logging audit action"
    );

    sqlx::query_as::<_, AuditLogRow>(
        r#"
        INSERT INTO audit_log (id, server_id, actor_id, actor_username, action, target_id, target_type, changes)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(id.as_i64())
    .bind(server_id.as_i64())
    .bind(actor_id.as_i64())
    .bind(actor_username)
    .bind(action)
    .bind(target_id)
    .bind(target_type)
    .bind(changes.map(sqlx::types::Json))
    .fetch_one(pool)
    .await
}

/// List audit log entries for a server with optional cursor-based pagination.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `server_id` - Snowflake ID of the server
/// * `limit` - Maximum number of entries to return
/// * `before_id` - Optional entry ID to paginate before (for older entries)
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_entries(
    pool: &PgPool,
    server_id: Snowflake,
    limit: i64,
    before_id: Option<i64>,
) -> Result<Vec<AuditLogRow>, sqlx::Error> {
    tracing::debug!(
        server_id = server_id.as_i64(),
        limit = limit,
        before_id = ?before_id,
        "listing audit log entries"
    );

    let query = if let Some(before) = before_id {
        sqlx::query_as::<_, AuditLogRow>(
            "SELECT * FROM audit_log WHERE server_id = $1 AND id < $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(server_id.as_i64())
        .bind(before)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, AuditLogRow>(
            "SELECT * FROM audit_log WHERE server_id = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(server_id.as_i64())
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    tracing::debug!(count = query.len(), "audit log entries fetched");
    Ok(query)
}
