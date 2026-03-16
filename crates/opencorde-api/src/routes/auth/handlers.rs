//! # Auth Handlers
//! HTTP request handlers for login and refresh endpoints.
//!
//! ## Endpoints
//! - POST /api/v1/auth/login — Login with email + password
//! - POST /api/v1/auth/refresh — Refresh access token via refresh_token cookie
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db::repos::user_repo (CRUD operations)
//! - opencorde_core::password (password hashing)
//! - opencorde_core::Snowflake (ID generation)
//! - crate::jwt (token creation)
//! - crate::AppState (database + config)
//! - crate::error::ApiError (unified error handling)
//! - super::types (request/response types)
//! - super::validation (input validation)

use anyhow;
use axum::{
    Json,
    extract::State,
    http::{HeaderValue, header::SET_COOKIE},
    response::{IntoResponse, Response},
};
use opencorde_core::{Snowflake, password};
use opencorde_db::repos::user_repo;

use super::types::{AuthResponse, LoginRequest, UserInfo};
use super::validation::{make_refresh_cookie, validate_login};
use crate::{AppState, error::ApiError, jwt};

/// POST /api/v1/auth/login — Authenticate user with email and password.
///
/// Verifies credentials and returns access + refresh tokens.
#[tracing::instrument(skip(state, req))]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    tracing::info!(email = %req.email, "login attempt");

    // Validate input
    validate_login(&req)?;

    // Find user by email
    let user_row = user_repo::get_by_email(&state.db, &req.email)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            tracing::warn!(email = %req.email, "user not found");
            ApiError::Unauthorized
        })?;

    tracing::debug!(user_id = user_row.id, "user found by email");

    // Verify password (password_hash is optional, but required for email login)
    let password_hash = user_row.password_hash.as_deref().ok_or_else(|| {
        tracing::warn!(user_id = user_row.id, "user has no password set");
        ApiError::Unauthorized
    })?;

    let password_valid = password::verify_password(&req.password, password_hash)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("password verification failed: {}", e)))?;

    if !password_valid {
        tracing::warn!(user_id = user_row.id, "invalid password");
        return Err(ApiError::Unauthorized);
    }

    tracing::debug!(user_id = user_row.id, "password verified");

    // Generate tokens
    let user_id = Snowflake::new(user_row.id);
    let access_token = jwt::create_access_token(
        user_id,
        &user_row.username,
        &state.config.jwt_secret,
        state.config.jwt_access_expiry,
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("token creation failed: {}", e)))?;

    let refresh_token = jwt::create_refresh_token(
        user_id,
        &user_row.username,
        &state.config.jwt_secret,
        state.config.jwt_refresh_expiry,
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("token creation failed: {}", e)))?;

    tracing::debug!(user_id = user_row.id, "tokens generated");

    // Build response
    let response = AuthResponse {
        user: UserInfo {
            id: user_row.id.to_string(),
            username: user_row.username,
            email: user_row.email.unwrap_or_default(),
        },
        access_token,
        expires_in: state.config.jwt_access_expiry,
    };

    // Build headers with refresh cookie
    let cookie = make_refresh_cookie(&refresh_token, state.config.jwt_refresh_expiry);
    let cookie_header = HeaderValue::from_str(&cookie)
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("invalid cookie header")))?;

    tracing::info!(user_id = user_row.id, "login successful");

    let mut response_obj = Json(response).into_response();
    response_obj.headers_mut().insert(SET_COOKIE, cookie_header);

    Ok(response_obj)
}

/// POST /api/v1/auth/refresh — Refresh access token using refresh_token cookie.
///
/// Validates refresh token and generates new access + refresh tokens.
#[tracing::instrument(skip(state, headers))]
pub async fn refresh(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Response, ApiError> {
    tracing::info!("token refresh attempt");

    // Extract refresh token from cookie
    let refresh_token = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookie_header| {
            // Parse "refresh_token=value; other=value" format
            cookie_header
                .split(';')
                .find(|part| part.trim().starts_with("refresh_token="))
                .map(|part| part.trim().strip_prefix("refresh_token=").unwrap_or(""))
        })
        .ok_or_else(|| {
            tracing::debug!("refresh_token cookie not found");
            ApiError::Unauthorized
        })?;

    if refresh_token.is_empty() {
        tracing::warn!("refresh_token cookie is empty");
        return Err(ApiError::Unauthorized);
    }

    tracing::debug!("refresh_token cookie extracted");

    // Validate refresh token
    let claims =
        jwt::validate_refresh_token(refresh_token, &state.config.jwt_secret).map_err(|_| {
            tracing::warn!("refresh token validation failed");
            ApiError::Unauthorized
        })?;

    // Parse user ID and fetch user to verify still exists
    let user_id = claims
        .sub
        .parse::<i64>()
        .map_err(|_| {
            tracing::warn!(sub = %claims.sub, "invalid user ID in token");
            ApiError::Unauthorized
        })
        .map(Snowflake::new)?;

    let user_row = user_repo::get_by_id(&state.db, user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            tracing::warn!(user_id = %user_id, "user not found for token refresh");
            ApiError::Unauthorized
        })?;

    tracing::debug!(user_id = %user_id, "user verified for token refresh");

    // Generate new tokens
    let new_access_token = jwt::create_access_token(
        user_id,
        &user_row.username,
        &state.config.jwt_secret,
        state.config.jwt_access_expiry,
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("token creation failed: {}", e)))?;

    let new_refresh_token = jwt::create_refresh_token(
        user_id,
        &user_row.username,
        &state.config.jwt_secret,
        state.config.jwt_refresh_expiry,
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("token creation failed: {}", e)))?;

    tracing::debug!(user_id = %user_id, "new tokens generated");

    // Build response
    let response = AuthResponse {
        user: UserInfo {
            id: user_row.id.to_string(),
            username: user_row.username,
            email: user_row.email.unwrap_or_default(),
        },
        access_token: new_access_token,
        expires_in: state.config.jwt_access_expiry,
    };

    // Build headers with new refresh cookie
    let cookie = make_refresh_cookie(&new_refresh_token, state.config.jwt_refresh_expiry);
    let cookie_header = HeaderValue::from_str(&cookie)
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("invalid cookie header")))?;

    tracing::info!(user_id = %user_id, "token refresh successful");

    let mut response_obj = Json(response).into_response();
    response_obj.headers_mut().insert(SET_COOKIE, cookie_header);

    Ok(response_obj)
}
