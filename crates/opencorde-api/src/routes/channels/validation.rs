//! # Channel Validation
//! Input validation for channel endpoints.
//!
//! ## Depends On
//! - opencorde_core::Snowflake
//! - crate::error::ApiError

use crate::error::ApiError;
use opencorde_core::Snowflake;

/// Validate channel name format.
pub fn validate_channel_name(name: &str) -> Result<(), ApiError> {
    if name.is_empty() || name.len() > 100 {
        return Err(ApiError::BadRequest(
            "channel name must be 1-100 characters".into(),
        ));
    }
    Ok(())
}

/// Validate channel type.
pub fn validate_channel_type(channel_type: i16) -> Result<(), ApiError> {
    if ![0, 1, 2].contains(&channel_type) {
        return Err(ApiError::BadRequest(
            "channel_type must be 0 (Text), 1 (Voice), or 2 (Category)".into(),
        ));
    }
    Ok(())
}

/// Parse a Snowflake ID from a string path parameter.
pub fn parse_snowflake_id(id_str: &str) -> Result<Snowflake, ApiError> {
    id_str
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("invalid id format".into()))
        .and_then(|id| {
            if id > 0 {
                Ok(Snowflake::new(id))
            } else {
                Err(ApiError::BadRequest("id must be positive".into()))
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_channel_name_valid() {
        assert!(validate_channel_name("general").is_ok());
        assert!(validate_channel_name("a").is_ok());
        assert!(validate_channel_name(&"x".repeat(100)).is_ok());
    }

    #[test]
    fn test_validate_channel_name_empty() {
        let result = validate_channel_name("");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_channel_name_too_long() {
        let long_name = "x".repeat(101);
        assert!(validate_channel_name(&long_name).is_err());
    }

    #[test]
    fn test_validate_channel_type_valid() {
        assert!(validate_channel_type(0).is_ok()); // Text
        assert!(validate_channel_type(1).is_ok()); // Voice
        assert!(validate_channel_type(2).is_ok()); // Category
    }

    #[test]
    fn test_validate_channel_type_invalid() {
        assert!(validate_channel_type(3).is_err());
        assert!(validate_channel_type(-1).is_err());
    }

    #[test]
    fn test_parse_snowflake_id_valid() {
        let result = parse_snowflake_id("123456");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_i64(), 123456);
    }

    #[test]
    fn test_parse_snowflake_id_invalid() {
        assert!(parse_snowflake_id("not_a_number").is_err());
        assert!(parse_snowflake_id("0").is_err());
        assert!(parse_snowflake_id("-1").is_err());
    }
}
