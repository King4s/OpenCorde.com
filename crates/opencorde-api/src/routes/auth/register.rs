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
use opencorde_db::repos::{invite_repo, user_repo};
use rand::RngCore;

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
            // Validate invite code against the database invite system
            let code = req.invite_code.as_deref().ok_or_else(|| {
                tracing::warn!("invite-only: no invite code provided");
                ApiError::BadRequest("INVALID_INVITE".into())
            })?;

            let invite = invite_repo::get_by_code(&state.db, code)
                .await
                .map_err(ApiError::Database)?
                .ok_or_else(|| {
                    tracing::warn!(code = %code, "invite-only: unknown invite code");
                    ApiError::BadRequest("INVALID_INVITE".into())
                })?;

            // Check expiry
            if let Some(expires_at) = invite.expires_at {
                if chrono::Utc::now() > expires_at {
                    tracing::warn!(code = %code, "invite-only: expired invite code");
                    return Err(ApiError::BadRequest("INVALID_INVITE".into()));
                }
            }

            // Check max uses
            if let Some(max) = invite.max_uses {
                if invite.uses >= max {
                    tracing::warn!(code = %code, uses = invite.uses, max, "invite-only: invite exhausted");
                    return Err(ApiError::BadRequest("INVALID_INVITE".into()));
                }
            }

            // Atomic: increment invite uses before proceeding
            invite_repo::increment_uses(&state.db, code)
                .await
                .map_err(|e| {
                    tracing::error!(code = %code, error = %e, "failed to increment invite uses");
                    ApiError::Database(e)
                })?;

            tracing::info!(code = %code, server_id = invite.server_id, "invite-only: valid invite accepted");
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

    // Generate email verification token and store it
    let verification_token = generate_verification_token();
    let verification_expires_at = chrono::Utc::now() + chrono::Duration::hours(24);
    let _ = sqlx::query(
        "UPDATE users SET email_verification_token = $1, email_verification_expires_at = $2 WHERE id = $3"
    )
    .bind(&verification_token)
    .bind(verification_expires_at)
    .bind(user_row.id)
    .execute(&state.db)
    .await;

    // Send verification email (log failures; don't block registration)
    if let Err(e) = state
        .email_service
        .send_verification_email(&req.email, &verification_token)
        .await
    {
        tracing::warn!(user_id = user_row.id, error = ?e, "failed to send verification email");
    }

    // Generate tokens
    let (refresh_token, jti) = jwt::create_refresh_token(
        user_id,
        &req.username,
        &state.config.jwt_secret,
        state.config.jwt_refresh_expiry,
        None,
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("token creation failed: {}", e)))?;

    let access_token = jwt::create_access_token(
        user_id,
        &req.username,
        &state.config.jwt_secret,
        state.config.jwt_access_expiry,
        None,
        Some(&jti),
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("token creation failed: {}", e)))?;

    // Store the JTI for rotation and theft detection
    let expires_at =
        chrono::Utc::now() + chrono::Duration::seconds(state.config.jwt_refresh_expiry as i64);
    opencorde_db::repos::refresh_token_repo::insert(&state.db, &jti, user_id.as_i64(), expires_at)
        .await
        .map_err(|e| {
            ApiError::Internal(anyhow::anyhow!("failed to store refresh token JTI: {}", e))
        })?;

    tracing::debug!(user_id = %user_id, "tokens generated and JTI stored");

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

/// Generate a random 32-byte (64 hex char) verification token.
fn generate_verification_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}
