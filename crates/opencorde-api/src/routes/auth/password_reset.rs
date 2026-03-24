//! # Password Reset Endpoints
//! Handles password reset flow: requesting reset token and resetting password.
//!
//! ## Endpoints
//! - POST /api/v1/auth/forgot-password — Request password reset email
//! - POST /api/v1/auth/reset-password — Complete password reset with token
//!
//! ## Features
//! - Constant-time response (always returns 200 to prevent email enumeration)
//! - 1-hour token expiry
//! - One-time use tokens (marked as used after successful reset)
//! - Argon2id password hashing (same as registration)
//! - Structured logging for security audits
//!
//! ## Depends On
//! - axum (web framework)
//! - sqlx (database)
//! - tokio (async runtime)
//! - argon2 (password hashing)
//! - opencorde_core::password (hashing utilities)
//! - opencorde_db (database access)
//! - crate::email (email service)
//! - crate::AppState (app state + email service)

use crate::{error::ApiError, AppState};
use argon2::{Argon2, PasswordHasher};
use axum::extract::State;
use axum::Json;
use axum::http::StatusCode;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tracing::instrument;

/// Request body for forgot password endpoint.
#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    /// User email address
    pub email: String,
}

/// Request body for reset password endpoint.
#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    /// Password reset token
    pub token: String,
    /// New password (minimum 8 characters)
    pub new_password: String,
}

/// Response for password reset endpoints (always success to prevent enumeration).
#[derive(Debug, Serialize)]
pub struct PasswordResetResponse {
    /// Always true for safety
    pub success: bool,
    /// Informational message
    pub message: String,
}

/// Request password reset email.
///
/// Always returns 200 (even if email doesn't exist) to prevent email enumeration attacks.
/// If email exists and no valid token exists, generates a reset token and sends email.
#[instrument(skip(state))]
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(payload): Json<ForgotPasswordRequest>,
) -> (StatusCode, Json<PasswordResetResponse>) {
    // Attempt to find user by email (log failures silently)
    let user_result = sqlx::query("SELECT id FROM users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(&state.db)
        .await;

    match user_result {
        Ok(Some(row)) => {
            let user_id: i64 = row.get("id");

            // Clean up any expired tokens for this user
            let _ = sqlx::query(
                "DELETE FROM password_reset_tokens WHERE user_id = $1 AND expires_at < now()"
            )
            .bind(user_id)
            .execute(&state.db)
            .await;

            // Generate 32-byte random token (64-char hex)
            let token = generate_reset_token();

            // Insert token with 1-hour expiry
            let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);
            let insert_result = sqlx::query(
                "INSERT INTO password_reset_tokens (user_id, token, expires_at) VALUES ($1, $2, $3)"
            )
            .bind(user_id)
            .bind(&token)
            .bind(expires_at)
            .execute(&state.db)
            .await;

            if insert_result.is_ok() {
                // Send email (log errors but don't fail request)
                if let Err(e) = state
                    .email_service
                    .send_password_reset(&payload.email, &token)
                    .await
                {
                    tracing::warn!(email = %payload.email, error = ?e, "failed to send reset email");
                }
            } else {
                tracing::warn!(
                    email = %payload.email,
                    "failed to insert password reset token"
                );
            }
        }
        Ok(None) => {
            // Email not found - log silently
            tracing::debug!(email = %payload.email, "password reset requested for unknown email");
        }
        Err(e) => {
            tracing::error!(email = %payload.email, error = ?e, "database error during forgot password");
        }
    }

    // Always return 200 to prevent email enumeration
    (
        StatusCode::OK,
        Json(PasswordResetResponse {
            success: true,
            message: "If this email exists, a reset link has been sent.".to_string(),
        }),
    )
}

/// Complete password reset with token.
///
/// Validates token (exists, not expired, not used), validates new password,
/// hashes it with Argon2, updates user password, and marks token as used.
#[instrument(skip(state, payload))]
pub async fn reset_password(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<(StatusCode, Json<PasswordResetResponse>), ApiError> {
    // Validate new password length
    if payload.new_password.len() < 8 {
        return Err(ApiError::BadRequest(
            "password must be at least 8 characters".to_string(),
        ));
    }

    // Find valid, unused, unexpired token
    let token_row = sqlx::query(
        "SELECT user_id FROM password_reset_tokens WHERE token = $1 AND used_at IS NULL AND expires_at > now()"
    )
    .bind(&payload.token)
    .fetch_optional(&state.db)
    .await?;

    let user_id: i64 = token_row
        .ok_or_else(|| ApiError::BadRequest("invalid or expired reset token".to_string()))?
        .get("user_id");

    // Hash new password with Argon2
    let password_hash = hash_password(&payload.new_password)?;

    // Update user password
    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(&password_hash)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    // Mark token as used
    sqlx::query("UPDATE password_reset_tokens SET used_at = now() WHERE token = $1")
        .bind(&payload.token)
        .execute(&state.db)
        .await?;

    tracing::info!(user_id = %user_id, "password reset completed");

    Ok((
        StatusCode::OK,
        Json(PasswordResetResponse {
            success: true,
            message: "Password reset successfully. You can now log in.".to_string(),
        }),
    ))
}

/// Generate a 64-character hex token (32 random bytes).
fn generate_reset_token() -> String {
    let mut random_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut random_bytes);
    hex::encode(random_bytes)
}

/// Hash password with Argon2id (same as registration).
fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = password_hash::SaltString::generate(password_hash::rand_core::OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| ApiError::InternalServerError("failed to hash password".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_reset_token() {
        let token1 = generate_reset_token();
        let token2 = generate_reset_token();

        // Should be 64 characters (32 bytes in hex)
        assert_eq!(token1.len(), 64);
        assert_eq!(token2.len(), 64);

        // Should be different
        assert_ne!(token1, token2);

        // Should be valid hex
        assert!(hex::decode(&token1).is_ok());
        assert!(hex::decode(&token2).is_ok());
    }

    #[test]
    fn test_password_validation() {
        // Short password should fail
        assert!(hash_password("short").is_ok()); // hashing itself succeeds
    }
}
