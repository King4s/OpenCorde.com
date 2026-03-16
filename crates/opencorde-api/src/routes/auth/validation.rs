//! # Auth Validation
//! Request validation helpers for authentication endpoints.
//!
//! ## Depends On
//! - crate::error::ApiError (error type)
//! - super::types (request types)

use super::types::{LoginRequest, RegisterRequest};
use crate::error::ApiError;

/// Validate registration request fields.
pub fn validate_register(req: &RegisterRequest) -> Result<(), ApiError> {
    // Validate username: 3-32 chars, alphanumeric + underscore
    if req.username.len() < 3 || req.username.len() > 32 {
        return Err(ApiError::BadRequest(
            "username must be 3-32 characters".into(),
        ));
    }

    if !req
        .username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_')
    {
        return Err(ApiError::BadRequest(
            "username must be alphanumeric or underscore".into(),
        ));
    }

    // Validate email: basic format check
    if !req.email.contains('@') || req.email.len() < 5 {
        return Err(ApiError::BadRequest("invalid email address".into()));
    }

    // Validate password: minimum 8 chars
    if req.password.len() < 8 {
        return Err(ApiError::BadRequest(
            "password must be at least 8 characters".into(),
        ));
    }

    Ok(())
}

/// Validate login request fields.
pub fn validate_login(req: &LoginRequest) -> Result<(), ApiError> {
    if req.email.is_empty() {
        return Err(ApiError::BadRequest("email is required".into()));
    }

    if req.password.is_empty() {
        return Err(ApiError::BadRequest("password is required".into()));
    }

    Ok(())
}

/// Create a refresh token cookie header value.
pub fn make_refresh_cookie(token: &str, max_age: u64) -> String {
    format!(
        "refresh_token={}; HttpOnly; Secure; SameSite=Strict; Path=/api/v1/auth; Max-Age={}",
        token, max_age
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_register_success() {
        let req = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "secure_password_123".to_string(),
        };
        assert!(validate_register(&req).is_ok());
    }

    #[test]
    fn test_validate_register_username_too_short() {
        let req = RegisterRequest {
            username: "ab".to_string(),
            email: "test@example.com".to_string(),
            password: "secure_password_123".to_string(),
        };
        assert!(validate_register(&req).is_err());
    }

    #[test]
    fn test_validate_register_username_too_long() {
        let req = RegisterRequest {
            username: "a".repeat(33),
            email: "test@example.com".to_string(),
            password: "secure_password_123".to_string(),
        };
        assert!(validate_register(&req).is_err());
    }

    #[test]
    fn test_validate_register_invalid_characters() {
        let req = RegisterRequest {
            username: "test-user".to_string(),
            email: "test@example.com".to_string(),
            password: "secure_password_123".to_string(),
        };
        assert!(validate_register(&req).is_err());
    }

    #[test]
    fn test_validate_register_valid_underscore() {
        let req = RegisterRequest {
            username: "test_user".to_string(),
            email: "test@example.com".to_string(),
            password: "secure_password_123".to_string(),
        };
        assert!(validate_register(&req).is_ok());
    }

    #[test]
    fn test_validate_register_invalid_email() {
        let req = RegisterRequest {
            username: "testuser".to_string(),
            email: "notanemail".to_string(),
            password: "secure_password_123".to_string(),
        };
        assert!(validate_register(&req).is_err());
    }

    #[test]
    fn test_validate_register_short_password() {
        let req = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "short".to_string(),
        };
        assert!(validate_register(&req).is_err());
    }

    #[test]
    fn test_validate_login_success() {
        let req = LoginRequest {
            email: "test@example.com".to_string(),
            password: "password".to_string(),
        };
        assert!(validate_login(&req).is_ok());
    }

    #[test]
    fn test_validate_login_empty_email() {
        let req = LoginRequest {
            email: "".to_string(),
            password: "password".to_string(),
        };
        assert!(validate_login(&req).is_err());
    }

    #[test]
    fn test_validate_login_empty_password() {
        let req = LoginRequest {
            email: "test@example.com".to_string(),
            password: "".to_string(),
        };
        assert!(validate_login(&req).is_err());
    }

    #[test]
    fn test_make_refresh_cookie() {
        let cookie = make_refresh_cookie("test_token_value", 604800);
        assert!(cookie.contains("refresh_token=test_token_value"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("Secure"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(cookie.contains("Max-Age=604800"));
    }
}
