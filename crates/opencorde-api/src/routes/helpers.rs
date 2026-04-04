//! # Route Helpers - Shared utilities for route handlers

use crate::error::ApiError;
use opencorde_core::Snowflake;
use sqlx::PgPool;

/// Parse a Snowflake ID from a path parameter string.
pub fn parse_snowflake(s: &str) -> Result<Snowflake, ApiError> {
    s.parse::<i64>()
        .map(Snowflake::new)
        .map_err(|_| ApiError::BadRequest("invalid id format".into()))
}

/// Check if a user is the server owner.
pub fn check_server_owner(user_id: Snowflake, owner_id: i64) -> Result<(), ApiError> {
    if user_id.as_i64() != owner_id {
        tracing::warn!(
            user_id = user_id.as_i64(),
            owner_id = owner_id,
            "user is not server owner"
        );
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

/// Check if a user meets the server's verification level requirement.
/// Called before join and before message send.
///
/// Levels:
/// 0 (NONE) — no check
/// 1 (LOW) — email must be verified
/// 2 (MEDIUM) — account must be > 5 minutes old
/// 3 (HIGH) — must be a server member for > 10 minutes
/// 4 (VERY_HIGH) — must have 2FA (totp_enabled)
#[tracing::instrument(skip(pool), fields(user_id = %user_id, server_id = %server_id))]
pub async fn check_verification_level(
    pool: &PgPool,
    user_id: Snowflake,
    server_id: Snowflake,
    include_member_tenure: bool,
) -> Result<(), ApiError> {
    use chrono::Utc;
    use opencorde_db::repos::{member_repo, server_repo, user_repo};

    let server = server_repo::get_by_id(pool, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;

    let level = server.verification_level;
    if level == 0 {
        return Ok(());
    }

    let user = user_repo::get_by_id(pool, user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("user not found".into()))?;

    if level >= 1 && !user.email_verified {
        tracing::warn!(user_id = user_id.as_i64(), "user email not verified");
        return Err(ApiError::Forbidden);
    }

    if level >= 2 {
        let age_secs = Utc::now()
            .signed_duration_since(user.created_at)
            .num_seconds();
        if age_secs < 300 {
            tracing::warn!(user_id = user_id.as_i64(), age_secs, "user account too new");
            return Err(ApiError::Forbidden);
        }
    }

    if level >= 3 && include_member_tenure {
        if let Some(member) = member_repo::get_member(pool, user_id, server_id)
            .await
            .map_err(ApiError::Database)?
        {
            let tenure_secs = Utc::now()
                .signed_duration_since(member.joined_at)
                .num_seconds();
            if tenure_secs < 600 {
                tracing::warn!(user_id = user_id.as_i64(), tenure_secs, "member tenure too short");
                return Err(ApiError::Forbidden);
            }
        }
    }

    if level >= 4 && !user.totp_enabled {
        tracing::warn!(user_id = user_id.as_i64(), "user 2FA not enabled");
        return Err(ApiError::Forbidden);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_snowflake_valid() {
        let result = parse_snowflake("123456789");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_i64(), 123456789);
    }

    #[test]
    fn test_parse_snowflake_invalid() {
        assert!(parse_snowflake("not_a_number").is_err());
    }

    #[test]
    fn test_check_server_owner_valid() {
        let result = check_server_owner(Snowflake::new(999), 999);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_server_owner_invalid() {
        let result = check_server_owner(Snowflake::new(111), 999);
        assert!(result.is_err());
    }
}
