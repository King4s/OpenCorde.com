//! # Register Handler
//! POST /api/v1/auth/register endpoint implementation.
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
    http::{HeaderValue, StatusCode, header::SET_COOKIE},
    response::{IntoResponse, Response},
};
use opencorde_core::{SnowflakeGenerator, generate_keypair, password};
use opencorde_db::repos::user_repo;

use super::types::{AuthResponse, RegisterRequest, UserInfo};
use super::validation::{make_refresh_cookie, validate_register};
use crate::{AppState, error::ApiError, jwt};

/// POST /api/v1/auth/register — Create a new user account.
///
/// Validates input, checks for conflicts, hashes password, and returns tokens.
#[tracing::instrument(skip(state, req))]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Response, ApiError> {
    tracing::info!(username = %req.username, "registration attempt");

    // Check registration mode
    use crate::config::RegistrationMode;
    match state.config.registration_mode {
        RegistrationMode::Closed => {
            tracing::warn!("registration attempt blocked: registration is closed");
            return Err(ApiError::Forbidden);
        }
        RegistrationMode::InviteOnly => {
            // TODO: validate invite code once invite system supports this mode
            tracing::warn!("invite-only mode: open registration blocked");
            return Err(ApiError::BadRequest("registration requires an invite code".into()));
        }
        RegistrationMode::Open => {}
    }

    // Validate input
    validate_register(&req)?;

    // Check if email already exists
    let existing_email = user_repo::get_by_email(&state.db, &req.email)
        .await
        .map_err(ApiError::Database)?;

    if existing_email.is_some() {
        tracing::warn!(email = %req.email, "email already exists");
        return Err(ApiError::Conflict("email already registered".into()));
    }

    // Check if username already exists
    let existing_username = user_repo::get_by_username(&state.db, &req.username)
        .await
        .map_err(ApiError::Database)?;

    if existing_username.is_some() {
        tracing::warn!(username = %req.username, "username already exists");
        return Err(ApiError::Conflict("username already taken".into()));
    }

    // Generate Ed25519 keypair for identity
    let (_private_key, public_key) = generate_keypair();
    tracing::debug!("Ed25519 keypair generated");

    // Hash password
    let password_hash = password::hash_password(&req.password)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("password hashing failed: {}", e)))?;

    tracing::debug!("password hashed");

    // Generate Snowflake ID for user
    let mut generator = SnowflakeGenerator::new(0, 0);
    let user_id = generator.next_id();

    tracing::debug!(user_id = %user_id, "snowflake ID generated");

    // Create user in database
    let user_row = user_repo::create_user(
        &state.db,
        user_id,
        &req.username,
        &public_key,
        Some(&req.email),
        Some(&password_hash),
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to create user");
        ApiError::Database(e)
    })?;

    tracing::info!(user_id = user_row.id, "user created successfully");

    // Generate tokens
    let access_token = jwt::create_access_token(
        user_id,
        &req.username,
        &state.config.jwt_secret,
        state.config.jwt_access_expiry,
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("token creation failed: {}", e)))?;

    let refresh_token = jwt::create_refresh_token(
        user_id,
        &req.username,
        &state.config.jwt_secret,
        state.config.jwt_refresh_expiry,
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("token creation failed: {}", e)))?;

    tracing::debug!(user_id = %user_id, "tokens generated");

    // Build response
    let response = AuthResponse {
        user: UserInfo {
            id: user_id.to_string(),
            username: req.username,
            email: req.email,
        },
        access_token,
        expires_in: state.config.jwt_access_expiry,
    };

    // Build headers with refresh cookie
    let cookie = make_refresh_cookie(&refresh_token, state.config.jwt_refresh_expiry);
    let cookie_header = HeaderValue::from_str(&cookie)
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("invalid cookie header")))?;

    tracing::info!(user_id = %user_id, "registration successful");

    let mut response_obj = (StatusCode::CREATED, Json(response)).into_response();
    response_obj.headers_mut().insert(SET_COOKIE, cookie_header);

    Ok(response_obj)
}
