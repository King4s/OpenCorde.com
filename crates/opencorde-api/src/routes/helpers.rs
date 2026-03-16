//! # Route Helpers - Shared utilities for route handlers

use crate::error::ApiError;
use opencorde_core::Snowflake;

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
