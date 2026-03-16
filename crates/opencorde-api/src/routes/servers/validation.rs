//! # Server Validation
//! Input validation for server endpoints.
//!
//! ## Depends On
//! - crate::error::ApiError

use crate::error::ApiError;

/// Validate server name format.
pub fn validate_server_name(name: &str) -> Result<(), ApiError> {
    if name.is_empty() || name.len() > 100 {
        return Err(ApiError::BadRequest(
            "server name must be 1-100 characters".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_server_name_valid() {
        assert!(validate_server_name("My Server").is_ok());
        assert!(validate_server_name("a").is_ok());
        assert!(validate_server_name(&"x".repeat(100)).is_ok());
    }

    #[test]
    fn test_validate_server_name_empty() {
        let result = validate_server_name("");
        assert!(result.is_err());
        match result {
            Err(ApiError::BadRequest(msg)) => {
                assert!(msg.contains("1-100 characters"));
            }
            _ => panic!("expected BadRequest error"),
        }
    }

    #[test]
    fn test_validate_server_name_too_long() {
        let long_name = "x".repeat(101);
        let result = validate_server_name(&long_name);
        assert!(result.is_err());
    }
}
