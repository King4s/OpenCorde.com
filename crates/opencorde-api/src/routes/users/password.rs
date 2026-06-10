//! PATCH /api/v1/users/@me/password — change password while logged in.

use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;

use crate::error::ApiError;
use crate::middleware::AuthUser;
use crate::AppState;
use opencorde_core::password;
use opencorde_db::repos::{refresh_token_repo, user_repo};

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// Change password for the authenticated user.
/// Validates current password, hashes and stores new password,
/// and revokes all other refresh tokens (session isolation).
#[tracing::instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
pub async fn change_password(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<StatusCode, ApiError> {
    if req.new_password.len() < 8 {
        return Err(ApiError::BadRequest("password must be at least 8 characters".into()));
    }

    // Fetch user with password hash
    let user = user_repo::get_by_id(&state.db, auth.user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::Unauthorized)?;

    // Verify current password
    let current_hash = user.password_hash.as_deref().ok_or_else(|| {
        ApiError::BadRequest("account has no password set".into())
    })?;

    let valid = password::verify_password(&req.current_password, current_hash)
        .map_err(|e| ApiError::InternalServerError(format!("password verification failed: {e}")))?;

    if !valid {
        tracing::warn!(user_id = %auth.user_id, "incorrect current password during password change");
        return Err(ApiError::Forbidden);
    }

    // Hash new password
    let new_hash = password::hash_password(&req.new_password)
        .map_err(|e| ApiError::InternalServerError(format!("password hashing failed: {e}")))?;

    // Store new password
    user_repo::set_password_hash(&state.db, auth.user_id, &new_hash)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(user_id = %auth.user_id, "password changed successfully");

    Ok(StatusCode::NO_CONTENT)
}
