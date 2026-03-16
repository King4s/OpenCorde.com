//! # Middleware: Authentication
//! JWT token extraction and validation from Authorization header.
//!
//! Provides the `AuthUser` extractor for Axum handlers that require authentication.
//!
//! ## Usage
//! Add `AuthUser` as a parameter to any handler that requires authentication:
//! ```ignore
//! async fn handler(auth: AuthUser) -> impl IntoResponse {
//!     let user_id = auth.user_id;
//!     let username = auth.username;
//!     // ...
//! }
//! ```
//!
//! The extractor automatically:
//! - Reads the Authorization header
//! - Strips the "Bearer " prefix
//! - Validates the JWT signature and expiration
//! - Extracts the user information from claims
//!
//! ## Errors
//! Returns `ApiError::Unauthorized` if:
//! - Authorization header is missing
//! - Bearer token is malformed
//! - Token signature is invalid
//! - Token is expired
//! - Token is not an access token
//!
//! ## Depends On
//! - axum (web framework and extractor traits)
//! - crate::jwt (token validation)
//! - crate::AppState (config access)
//! - crate::error::ApiError (error type)
//! - opencorde_core::Snowflake (user ID type)

use std::sync::Arc;

use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use opencorde_core::Snowflake;
use tracing::instrument;

use crate::{config::Config, error::ApiError, jwt};

/// Authenticated user info extracted from JWT.
///
/// Automatically extracted from the Authorization header by Axum.
/// Contains the user ID and username from the validated token.
#[derive(Debug, Clone)]
pub struct AuthUser {
    /// Snowflake user ID
    pub user_id: Snowflake,
    /// Username (from token claims, for convenience)
    pub username: String,
}

impl<S> FromRequestParts<S> for AuthUser
where
    Arc<Config>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    #[instrument(skip_all, err)]
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract the Authorization header
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(ApiError::Unauthorized)
            .inspect_err(|_| tracing::debug!("missing or invalid Authorization header"))?;

        // Strip "Bearer " prefix
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(ApiError::Unauthorized)
            .inspect_err(|_| {
                tracing::debug!("Authorization header missing Bearer prefix");
            })?;

        // Extract config to get JWT secret
        let config = Arc::<Config>::from_ref(state);

        // Validate the token and extract claims
        let claims = jwt::validate_access_token(token, &config.jwt_secret).map_err(|_| {
            tracing::debug!("JWT validation failed");
            ApiError::Unauthorized
        })?;

        // Parse user_id from claims
        let user_id = claims
            .sub
            .parse::<i64>()
            .map_err(|_| {
                tracing::warn!(sub = %claims.sub, "failed to parse user ID from token claims");
                ApiError::Unauthorized
            })
            .map(Snowflake::new)?;

        tracing::debug!(user_id = %user_id, username = %claims.username, "user authenticated");

        Ok(AuthUser {
            user_id,
            username: claims.username,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_user_creation() {
        let user_id = Snowflake::new(123456789);
        let username = "testuser".to_string();

        let auth = AuthUser {
            user_id,
            username: username.clone(),
        };

        assert_eq!(auth.user_id.as_i64(), 123456789);
        assert_eq!(auth.username, username);
    }

    #[test]
    fn test_auth_user_clone() {
        let user_id = Snowflake::new(999);
        let auth1 = AuthUser {
            user_id,
            username: "user".to_string(),
        };

        let auth2 = auth1.clone();

        assert_eq!(auth1.user_id, auth2.user_id);
        assert_eq!(auth1.username, auth2.username);
    }
}
