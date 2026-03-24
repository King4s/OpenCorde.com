//! # Repository: Channel Permission Overrides
//! CRUD operations for per-channel permission overrides (role and member).
//!
//! Each override has allow_bits and deny_bits. Bits not in either inherit
//! from the member's role-based permissions.
//!
//! ## Depends On
//! - sqlx::PgPool
//! - opencorde_core::snowflake (not used here; caller passes raw i64)

use sqlx::PgPool;

/// Row type for reading a channel permission override from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OverrideRow {
    pub id: i64,
    pub channel_id: i64,
    /// "role" or "member"
    pub target_type: String,
    pub target_id: i64,
    pub allow_bits: i64,
    pub deny_bits: i64,
}

/// List all permission overrides for a channel, ordered by target_type then target_id.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_for_channel(
    pool: &PgPool,
    channel_id: i64,
) -> Result<Vec<OverrideRow>, sqlx::Error> {
    tracing::info!(channel_id = channel_id, "listing channel permission overrides");

    sqlx::query_as::<_, OverrideRow>(
        "SELECT * FROM channel_permission_overrides \
         WHERE channel_id = $1 \
         ORDER BY target_type DESC, target_id ASC",
    )
    .bind(channel_id)
    .fetch_all(pool)
    .await
}

/// Insert or update a channel permission override.
///
/// Uses ON CONFLICT to upsert by (channel_id, target_type, target_id).
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn upsert(
    pool: &PgPool,
    channel_id: i64,
    target_type: &str,
    target_id: i64,
    allow_bits: i64,
    deny_bits: i64,
) -> Result<OverrideRow, sqlx::Error> {
    tracing::info!(
        channel_id = channel_id,
        target_type = %target_type,
        target_id = target_id,
        allow_bits = allow_bits,
        deny_bits = deny_bits,
        "upserting channel permission override"
    );

    let row = sqlx::query_as::<_, OverrideRow>(
        "INSERT INTO channel_permission_overrides \
             (channel_id, target_type, target_id, allow_bits, deny_bits) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (channel_id, target_type, target_id) \
         DO UPDATE SET allow_bits = EXCLUDED.allow_bits, \
                       deny_bits  = EXCLUDED.deny_bits \
         RETURNING *",
    )
    .bind(channel_id)
    .bind(target_type)
    .bind(target_id)
    .bind(allow_bits)
    .bind(deny_bits)
    .fetch_one(pool)
    .await?;

    tracing::info!(override_id = row.id, "channel permission override upserted");
    Ok(row)
}

/// Delete a channel permission override.
///
/// Silently succeeds if the override does not exist.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn delete(
    pool: &PgPool,
    channel_id: i64,
    target_type: &str,
    target_id: i64,
) -> Result<(), sqlx::Error> {
    tracing::info!(
        channel_id = channel_id,
        target_type = %target_type,
        target_id = target_id,
        "deleting channel permission override"
    );

    sqlx::query(
        "DELETE FROM channel_permission_overrides \
         WHERE channel_id = $1 AND target_type = $2 AND target_id = $3",
    )
    .bind(channel_id)
    .bind(target_type)
    .bind(target_id)
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_override_row_fields() {
        let row = OverrideRow {
            id: 1,
            channel_id: 100,
            target_type: "role".to_string(),
            target_id: 200,
            allow_bits: 0x10,
            deny_bits: 0x04,
        };
        assert_eq!(row.target_type, "role");
        assert_eq!(row.allow_bits, 0x10);
        assert_eq!(row.deny_bits, 0x04);
    }
}
