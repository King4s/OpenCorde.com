//! # JWT Token Management
//! Create and validate JWT access and refresh tokens.
//!
//! ## Token Types
//! - Access token: short-lived (15 min default), sent in Authorization header
//! - Refresh token: long-lived (7 days default), sent as HttpOnly cookie
//!
//! ## Claims
//! All tokens include standard JWT claims (iat, exp) plus:
//! - `sub` — User ID (as string)
//! - `username` — Username (for convenience)
//! - `token_type` — Either "access" or "refresh"
//! - `jti` — JWT ID (UUID v4, refresh tokens only) used for rotation and theft detection
//!
//! ## Features
//! - `create_access_token` — Generate short-lived access token
//! - `create_refresh_token` — Generate long-lived refresh token; returns (token, jti)
//! - `validate_token` — Validate any token and extract claims
//! - `validate_access_token` — Validate and ensure token is access type
//! - `validate_refresh_token` — Validate and ensure token is refresh type
//!
//! ## Depends On
//! - jsonwebtoken crate (workspace dependency)
//! - chrono crate (workspace dependency)
//! - uuid crate (workspace dependency) — for JTI generation
//! - opencorde_core::Snowflake

use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;

use opencorde_core::Snowflake;

/// JWT Claims for both access and refresh tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — user ID as string
    pub sub: String,
    /// Username (for convenience, not cryptographically guaranteed)
    pub username: String,
    /// Token type: "access" or "refresh"
    pub token_type: String,
    /// JWT ID — UUID v4, present on refresh tokens only.
    /// Used to track issued tokens in the DB for rotation and theft detection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expires at (Unix timestamp)
    pub exp: i64,
}

/// Create a JWT access token.
///
/// # Arguments
/// * `user_id` — Snowflake user ID
/// * `username` — Username for logging/debugging
/// * `secret` — JWT signing secret
/// * `expiry_seconds` — Token lifetime in seconds
///
/// # Returns
/// The encoded JWT token as a string.
///
/// # Errors
/// Returns `jsonwebtoken::errors::Error` if encoding fails.
#[instrument(skip(secret), fields(user_id = %user_id, username = %username))]
pub fn create_access_token(
    user_id: Snowflake,
    username: &str,
    secret: &str,
    expiry_seconds: u64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.as_i64().to_string(),
        username: username.to_string(),
        token_type: "access".to_string(),
        jti: None,
        iat: now.timestamp(),
        exp: (now + Duration::seconds(expiry_seconds as i64)).timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .inspect(|_token| {
        tracing::debug!(expiry_seconds, "access token created");
    })
}

/// Create a JWT refresh token.
///
/// Generates a UUID v4 JTI which must be stored in the `refresh_tokens` table
/// by the caller. The JTI is embedded in the token claims and also returned
/// separately so the caller can persist it without re-decoding the token.
///
/// # Arguments
/// * `user_id` — Snowflake user ID
/// * `username` — Username for logging/debugging
/// * `secret` — JWT signing secret
/// * `expiry_seconds` — Token lifetime in seconds
///
/// # Returns
/// A tuple `(token, jti)` where `token` is the encoded JWT string and
/// `jti` is the UUID v4 identifier embedded in the token.
///
/// # Errors
/// Returns `jsonwebtoken::errors::Error` if encoding fails.
#[instrument(skip(secret), fields(user_id = %user_id, username = %username))]
pub fn create_refresh_token(
    user_id: Snowflake,
    username: &str,
    secret: &str,
    expiry_seconds: u64,
) -> Result<(String, String), jsonwebtoken::errors::Error> {
    let jti = Uuid::new_v4().to_string();
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.as_i64().to_string(),
        username: username.to_string(),
        token_type: "refresh".to_string(),
        jti: Some(jti.clone()),
        iat: now.timestamp(),
        exp: (now + Duration::seconds(expiry_seconds as i64)).timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .inspect(|_| {
        tracing::debug!(expiry_seconds, "refresh token created");
    })?;

    Ok((token, jti))
}

/// Validate a JWT token and return the claims.
///
/// Verifies the signature and expiration.
///
/// # Arguments
/// * `token` — The JWT token string
/// * `secret` — The JWT signing secret (must match the one used for encoding)
///
/// # Returns
/// The decoded claims if valid.
///
/// # Errors
/// Returns `jsonwebtoken::errors::Error` if:
/// - Token signature is invalid
/// - Token is expired
/// - Token is malformed
#[instrument(skip(token, secret))]
pub fn validate_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    tracing::debug!(username = %token_data.claims.username, "token validated");
    Ok(token_data.claims)
}

/// Validate a token and verify it's an access token.
///
/// # Arguments
/// * `token` — The JWT token string
/// * `secret` — The JWT signing secret
///
/// # Returns
/// The claims if the token is valid and is an access token.
///
/// # Errors
/// Returns `jsonwebtoken::errors::Error` if validation fails or token type is not "access".
#[instrument(skip(token, secret))]
pub fn validate_access_token(
    token: &str,
    secret: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let claims = validate_token(token, secret)?;
    if claims.token_type != "access" {
        tracing::warn!(token_type = %claims.token_type, "invalid token type for access");
        return Err(jsonwebtoken::errors::ErrorKind::InvalidToken.into());
    }
    Ok(claims)
}

/// Validate a token and verify it's a refresh token.
///
/// # Arguments
/// * `token` — The JWT token string
/// * `secret` — The JWT signing secret
///
/// # Returns
/// The claims if the token is valid and is a refresh token.
///
/// # Errors
/// Returns `jsonwebtoken::errors::Error` if validation fails or token type is not "refresh".
#[instrument(skip(token, secret))]
pub fn validate_refresh_token(
    token: &str,
    secret: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let claims = validate_token(token, secret)?;
    if claims.token_type != "refresh" {
        tracing::warn!(token_type = %claims.token_type, "invalid token type for refresh");
        return Err(jsonwebtoken::errors::ErrorKind::InvalidToken.into());
    }
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "test-secret-key-min-32-chars-long!!!";

    #[test]
    fn test_access_token_roundtrip() {
        let uid = Snowflake::new(123);
        let tok = create_access_token(uid, "user", SECRET, 3600).unwrap();
        let c = validate_access_token(&tok, SECRET).unwrap();
        assert_eq!(c.sub, "123");
        assert_eq!(c.token_type, "access");
    }

    #[test]
    fn test_refresh_token_roundtrip() {
        let uid = Snowflake::new(456);
        let (tok, jti) = create_refresh_token(uid, "user", SECRET, 604800).unwrap();
        assert!(!jti.is_empty());
        let c = validate_refresh_token(&tok, SECRET).unwrap();
        assert_eq!(c.sub, "456");
        assert_eq!(c.token_type, "refresh");
        assert_eq!(c.jti.as_deref(), Some(jti.as_str()));
    }

    #[test]
    fn test_access_token_has_no_jti() {
        let uid = Snowflake::new(1);
        let tok = create_access_token(uid, "u", SECRET, 3600).unwrap();
        let c = validate_access_token(&tok, SECRET).unwrap();
        assert!(c.jti.is_none());
    }

    #[test]
    fn test_type_enforcement() {
        let uid = Snowflake::new(1);
        let access = create_access_token(uid, "u", SECRET, 3600).unwrap();
        let (refresh, _jti) = create_refresh_token(uid, "u", SECRET, 3600).unwrap();
        assert!(validate_refresh_token(&access, SECRET).is_err());
        assert!(validate_access_token(&refresh, SECRET).is_err());
    }

    #[test]
    fn test_wrong_secret() {
        let uid = Snowflake::new(1);
        let tok = create_access_token(uid, "u", SECRET, 3600).unwrap();
        assert!(validate_token(&tok, "wrong-secret-key-min-32-chars!!!!!").is_err());
    }

    #[test]
    fn test_malformed_token() {
        assert!(validate_token("not.valid", SECRET).is_err());
    }
}
