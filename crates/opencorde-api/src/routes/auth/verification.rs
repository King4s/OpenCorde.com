//! # Email Verification Endpoints
//! Handles email verification after registration.
//!
//! ## Endpoints
//! - GET  /api/v1/auth/verify-email         — Verify email with token from link
//! - POST /api/v1/auth/resend-verification  — Resend verification email (requires auth)
//!
//! ## Flow
//! 1. On register, a 24-hour token is stored in users.email_verification_token
//! 2. User clicks link → GET verify-email?token=... → sets email_verified=true
//! 3. If link expired or lost → POST resend-verification (authed) → new token+email
//!
//! ## Depends On
//! - axum (web framework)
//! - sqlx (database)
//! - chrono (token expiry)
//! - rand + hex (token generation)
//! - crate::email (email service)
//! - crate::middleware::auth::AuthUser (resend endpoint)
//! - crate::AppState (app state)

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tracing::instrument;

/// Query parameters for the verify-email endpoint.
#[derive(Debug, Deserialize)]
pub struct VerifyEmailQuery {
    /// Verification token from the email link
    pub token: String,
}

/// Generic success response.
#[derive(Debug, Serialize)]
pub struct VerificationResponse {
    pub success: bool,
    pub message: String,
}

/// GET /api/v1/auth/verify-email?token=... — Verify email address.
///
/// Finds the user by token, checks expiry, sets email_verified=true, clears token.
/// Returns 400 if the token is invalid or expired.
#[instrument(skip(state, query), fields(token = %query.token.chars().take(8).collect::<String>()))]
pub async fn verify_email(
    State(state): State<AppState>,
    Query(query): Query<VerifyEmailQuery>,
) -> Result<(StatusCode, Json<VerificationResponse>), ApiError> {
    let row = sqlx::query(
        r#"
        UPDATE users
        SET email_verified = TRUE,
            email_verification_token = NULL,
            email_verification_expires_at = NULL
        WHERE email_verification_token = $1
          AND email_verification_expires_at > NOW()
          AND email_verified = FALSE
        RETURNING id
        "#,
    )
    .bind(&query.token)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?;

    match row {
        Some(r) => {
            let user_id: i64 = r.get("id");
            tracing::info!(user_id, "email verified");
            Ok((
                StatusCode::OK,
                Json(VerificationResponse {
                    success: true,
                    message: "Email verified successfully.".to_string(),
                }),
            ))
        }
        None => Err(ApiError::BadRequest(
            "Invalid or expired verification token.".to_string(),
        )),
    }
}

/// POST /api/v1/auth/resend-verification — Resend verification email.
///
/// Generates a new 24-hour token and sends a fresh verification email.
/// Returns 400 if the email is already verified.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn resend_verification(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<(StatusCode, Json<VerificationResponse>), ApiError> {
    // Fetch user email and current verified status
    let row = sqlx::query(
        "SELECT email, email_verified FROM users WHERE id = $1",
    )
    .bind(auth.user_id.as_i64())
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or(ApiError::Unauthorized)?;

    let already_verified: bool = row.get("email_verified");
    if already_verified {
        return Err(ApiError::BadRequest("Email is already verified.".to_string()));
    }

    let email: Option<String> = row.try_get("email").ok();
    let email = email.ok_or_else(|| ApiError::BadRequest("No email address on account.".to_string()))?;

    // Generate new token
    let token = generate_verification_token();
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

    sqlx::query(
        "UPDATE users SET email_verification_token = $1, email_verification_expires_at = $2 WHERE id = $3",
    )
    .bind(&token)
    .bind(expires_at)
    .bind(auth.user_id.as_i64())
    .execute(&state.db)
    .await
    .map_err(ApiError::Database)?;

    if let Err(e) = state.email_service.send_verification_email(&email, &token).await {
        tracing::warn!(user_id = %auth.user_id, error = ?e, "failed to send verification email");
    }

    tracing::info!(user_id = %auth.user_id, "verification email resent");
    Ok((
        StatusCode::OK,
        Json(VerificationResponse {
            success: true,
            message: "If your email is not yet verified, a new link has been sent.".to_string(),
        }),
    ))
}

/// Generate a random 64-character hex verification token.
fn generate_verification_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_length() {
        let t1 = generate_verification_token();
        let t2 = generate_verification_token();
        assert_eq!(t1.len(), 64);
        assert_ne!(t1, t2);
    }
}
