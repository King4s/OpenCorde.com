//! # Permission Check Utilities
//! Async helpers that verify a user holds the required permission flag
//! before an operation proceeds.
//!
//! ## Algorithm
//! 1. Server owner always passes (unconditional bypass).
//! 2. OR together all of the user's role permission bits → base permissions.
//! 3. If ADMINISTRATOR bit is set → pass.
//! 4. For channel checks, apply channel-level allow/deny overrides via
//!    `opencorde_core::permissions::compute_permissions`.
//! 5. If required flag still not set → return `ApiError::Forbidden`.
//!
//! ## Depends On
//! - opencorde_core::permissions (Permissions, PermissionOverwrite, OverwriteType, compute_permissions)
//! - opencorde_db::repos (channel_repo, server_repo, role_repo, member_repo, channel_override_repo)

use opencorde_core::{
    Snowflake,
    permissions::{OverwriteType, PermissionOverwrite, Permissions, compute_permissions},
};
use opencorde_db::repos::{channel_override_repo, member_repo, role_repo, server_repo};
use sqlx::PgPool;

use crate::error::ApiError;

/// Require the calling user to hold `required` in the given server.
///
/// Server owner always passes. Non-members have only `default_everyone` bits.
///
/// # Errors
/// - `ApiError::NotFound` if the server does not exist.
/// - `ApiError::Forbidden` if the user lacks the required permission.
/// - `ApiError::Database` on database failure.
#[tracing::instrument(skip(pool), fields(user_id = user_id.as_i64(), server_id = server_id.as_i64()))]
pub async fn require_server_perm(
    pool: &PgPool,
    user_id: Snowflake,
    server_id: Snowflake,
    required: Permissions,
) -> Result<(), ApiError> {
    let server = server_repo::get_by_id(pool, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;

    // Owner always has every permission
    if server.owner_id == user_id.as_i64() {
        return Ok(());
    }

    let effective = compute_base_perms(pool, user_id, server_id).await?;

    if effective.contains(Permissions::ADMINISTRATOR) || effective.contains(required) {
        return Ok(());
    }

    tracing::warn!(required = ?required, "server permission denied");
    Err(ApiError::Forbidden)
}

/// Require the calling user to hold `required` in the given channel.
///
/// Resolves the channel's server, then applies channel overrides on top of
/// the user's role-based base permissions.
///
/// # Errors
/// - `ApiError::NotFound` if the channel or server does not exist.
/// - `ApiError::Forbidden` if the user lacks the required permission.
/// - `ApiError::Database` on database failure.
#[tracing::instrument(skip(pool), fields(user_id = user_id.as_i64(), channel_id = channel_id.as_i64()))]
pub async fn require_channel_perm(
    pool: &PgPool,
    user_id: Snowflake,
    channel_id: Snowflake,
    required: Permissions,
) -> Result<(), ApiError> {
    // Resolve channel → server_id
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT server_id FROM channels WHERE id = $1")
            .bind(channel_id.as_i64())
            .fetch_optional(pool)
            .await
            .map_err(ApiError::Database)?;

    let (server_id_raw,) = row
        .ok_or_else(|| ApiError::NotFound("channel not found".into()))?;
    let server_id = Snowflake::new(server_id_raw);

    // Owner bypass
    let server = server_repo::get_by_id(pool, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;

    if server.owner_id == user_id.as_i64() {
        return Ok(());
    }

    let base = compute_base_perms(pool, user_id, server_id).await?;

    if base.contains(Permissions::ADMINISTRATOR) {
        return Ok(());
    }

    // Get user's role IDs for overwrite matching
    let member_roles = member_repo::list_member_roles(pool, user_id, server_id)
        .await
        .map_err(ApiError::Database)?;
    let role_ids: Vec<Snowflake> = member_roles
        .iter()
        .map(|r| Snowflake::new(r.role_id))
        .collect();

    // Load and convert channel permission overrides
    let raw = channel_override_repo::list_for_channel(pool, channel_id.as_i64())
        .await
        .map_err(ApiError::Database)?;

    let overwrites: Vec<PermissionOverwrite> = raw
        .into_iter()
        .map(|row| PermissionOverwrite {
            id: Snowflake::new(row.target_id),
            target_type: if row.target_type == "role" {
                OverwriteType::Role
            } else {
                OverwriteType::Member
            },
            allow: Permissions::from_bits_truncate(row.allow_bits as u64),
            deny: Permissions::from_bits_truncate(row.deny_bits as u64),
        })
        .collect();

    let effective = compute_permissions(base, &overwrites, user_id, &role_ids);

    if effective.contains(required) {
        return Ok(());
    }

    tracing::warn!(required = ?required, "channel permission denied");
    Err(ApiError::Forbidden)
}

/// Compute a user's server-level permissions by OR-ing all their role bits.
///
/// Falls back to `Permissions::default_everyone()` when the user has no roles.
async fn compute_base_perms(
    pool: &PgPool,
    user_id: Snowflake,
    server_id: Snowflake,
) -> Result<Permissions, ApiError> {
    let member_roles = member_repo::list_member_roles(pool, user_id, server_id)
        .await
        .map_err(ApiError::Database)?;

    if member_roles.is_empty() {
        return Ok(Permissions::default_everyone());
    }

    let user_role_ids: Vec<i64> = member_roles.iter().map(|r| r.role_id).collect();

    let all_roles = role_repo::list_by_server(pool, server_id)
        .await
        .map_err(ApiError::Database)?;

    let base = all_roles
        .iter()
        .filter(|r| user_role_ids.contains(&r.id))
        .fold(Permissions::default_everyone(), |acc, role| {
            acc | Permissions::from_bits_truncate(role.permissions as u64)
        });

    Ok(base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permissions_flag_bits() {
        // Sanity: bits are Discord-compatible
        assert_eq!(Permissions::ADMINISTRATOR.bits(), 1 << 3);
        assert_eq!(Permissions::VIEW_CHANNEL.bits(), 1 << 10);
        assert_eq!(Permissions::SEND_MESSAGES.bits(), 1 << 11);
        assert_eq!(Permissions::BAN_MEMBERS.bits(), 1 << 2);
        assert_eq!(Permissions::MANAGE_CHANNELS.bits(), 1 << 4);
    }
}
