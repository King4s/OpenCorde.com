//! # Auth Types
//! Request and response types for authentication endpoints.
//!
//! ## Depends On
//! - serde (JSON serialization)

use serde::{Deserialize, Serialize};

/// Request body for user registration.
#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterRequest {
    /// Username: 3-32 alphanumeric characters + underscore
    pub username: String,
    /// Email address
    pub email: String,
    /// Password: minimum 8 characters
    pub password: String,
    /// Invite code — required when REGISTRATION_MODE=invite_only
    pub invite_code: Option<String>,
}

/// Request body for user login.
#[derive(Debug, Deserialize, Serialize)]
pub struct LoginRequest {
    /// Email address
    pub email: String,
    /// Password
    pub password: String,
    /// TOTP code from authenticator app — required if the account has 2FA enabled.
    /// If missing and 2FA is enabled, the server returns 403 TWO_FACTOR_REQUIRED.
    pub totp_code: Option<String>,
}

/// Response body for authentication (register/login/refresh).
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    /// Authenticated user info
    pub user: UserInfo,
    /// JWT access token
    pub access_token: String,
    /// Access token expiry in seconds
    pub expires_in: u64,
}

/// User information in authentication response.
#[derive(Debug, Serialize)]
pub struct UserInfo {
    /// Snowflake user ID
    pub id: String,
    /// Username
    pub username: String,
    /// Email address
    pub email: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_response_serialization() {
        let response = AuthResponse {
            user: UserInfo {
                id: "123456".to_string(),
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
            },
            access_token: "token_value".to_string(),
            expires_in: 900,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("123456"));
        assert!(json.contains("testuser"));
        assert!(json.contains("test@example.com"));
        assert!(json.contains("token_value"));
        assert!(json.contains("900"));
    }

    #[test]
    fn test_register_request_serialization() {
        let req = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            invite_code: None,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("testuser"));
        assert!(json.contains("test@example.com"));
    }

    #[test]
    fn test_login_request_serialization() {
        let req = LoginRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            totp_code: None,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("test@example.com"));
    }
}
