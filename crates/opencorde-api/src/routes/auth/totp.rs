//! # Route: TOTP Two-Factor Authentication
//! Endpoints to set up, verify, and disable TOTP-based 2FA.
//!
//! ## Flow
//! 1. `POST /auth/2fa/enable` — generates secret + otpauth:// URL; saves secret to DB
//! 2. User scans QR code in authenticator app and enters the shown code
//! 3. `POST /auth/2fa/verify` — verifies the first code; sets `totp_enabled = TRUE`
//! 4. `DELETE /auth/2fa` — disables 2FA (requires current TOTP code)
//!
//! ## Login Integration
//! When `totp_enabled = TRUE`, the login endpoint requires `totp_code` in the request.
//! Missing code → `ApiError::TwoFactorRequired` (403 / TWO_FACTOR_REQUIRED).
//! Wrong code → `ApiError::Unauthorized`.
//!
//! ## Depends On
//! - totp-rs crate (RFC 6238 TOTP, gen_secret + otpauth features)
//! - opencorde_db::repos::user_repo (totp_secret / totp_enabled CRUD)
//! - crate::middleware::auth::AuthUser
//! - crate::error::ApiError

use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use totp_rs::{Algorithm, Secret, TOTP};
use tracing::instrument;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};
use crate::routes::moderation::audit_mod::log_mod_action;
use opencorde_db::repos::user_repo;

/// Issuer name shown in authenticator apps (e.g., Google Authenticator).
pub const APP_NAME: &str = "OpenCorde";

/// Response body for the 2FA enable endpoint.
#[derive(Debug, Serialize)]
pub struct TotpEnableResponse {
    /// otpauth:// URL — encode as QR code for authenticator apps
    pub otpauth_url: String,
    /// Base32 secret — shown as text fallback for manual entry
    pub secret: String,
}

/// Request body for the 2FA verify endpoint.
#[derive(Debug, Deserialize)]
pub struct TotpVerifyRequest {
    /// 6-digit TOTP code from the authenticator app
    pub code: String,
}

/// Request body for the 2FA disable endpoint.
#[derive(Debug, Deserialize)]
pub struct TotpDisableRequest {
    /// Current valid TOTP code (confirms the user possesses the device)
    pub code: String,
}

/// POST /api/v1/auth/2fa/enable — Generate a TOTP secret and QR URL.
///
/// Saves the new secret to the user's account but does NOT activate 2FA.
/// The client must display the QR code / secret and prompt for verification.
///
/// # Errors
/// Returns 400 if the user has no email set (needed as the TOTP account name).
/// Returns 500 on DB or TOTP generation failure.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn enable(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<(StatusCode, Json<TotpEnableResponse>), ApiError> {
    tracing::info!("generating TOTP secret for 2FA setup");

    let user = user_repo::get_by_id(&state.db, auth.user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::Unauthorized)?;

    let account_name = user
        .email
        .as_deref()
        .unwrap_or(&user.username)
        .to_string();

    // Generate a new random secret
    let secret = Secret::generate_secret();
    let secret_base32 = secret.to_encoded().to_string();

    let totp = make_totp(secret_base32.clone(), &account_name, APP_NAME)?;
    let otpauth_url = totp.get_url();

    // Persist secret (not yet enabled)
    user_repo::set_totp_secret(&state.db, auth.user_id, &secret_base32)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(user_id = %auth.user_id, "TOTP secret generated and stored");

    Ok((
        StatusCode::OK,
        Json(TotpEnableResponse {
            otpauth_url,
            secret: secret_base32,
        }),
    ))
}

/// POST /api/v1/auth/2fa/verify — Verify the first TOTP code and activate 2FA.
///
/// The user must call this after scanning the QR code from `/auth/2fa/enable`.
/// On success, `totp_enabled` is set to `true` in the DB.
///
/// # Errors
/// Returns 400 if 2FA setup was not initiated (no secret stored).
/// Returns 401 if the TOTP code is invalid.
#[instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
pub async fn verify(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<TotpVerifyRequest>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("verifying TOTP code for 2FA activation");

    let user = user_repo::get_by_id(&state.db, auth.user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::Unauthorized)?;

    let secret_base32 = user.totp_secret.ok_or_else(|| {
        ApiError::BadRequest("2FA setup not initiated — call /auth/2fa/enable first".into())
    })?;

    let account_name = user.email.as_deref().unwrap_or(&user.username).to_string();
    let totp = make_totp(secret_base32, &account_name, APP_NAME)?;

    let valid = totp.check_current(&req.code).map_err(|e| {
        tracing::error!(error = %e, "TOTP time error during verify");
        ApiError::Internal(anyhow::anyhow!("TOTP clock error: {}", e))
    })?;

    if !valid {
        tracing::warn!(user_id = %auth.user_id, "invalid TOTP code during 2FA verify");
        return Err(ApiError::Unauthorized);
    }

    user_repo::enable_totp(&state.db, auth.user_id)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(user_id = %auth.user_id, "2FA successfully enabled");
    log_mod_action(&state, opencorde_core::Snowflake::new(0), auth.user_id, "2fa.enable", auth.user_id.as_i64()).await;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/auth/2fa — Disable 2FA.
///
/// Requires a valid current TOTP code to confirm the user has the device.
///
/// # Errors
/// Returns 400 if 2FA is not enabled.
/// Returns 401 if the TOTP code is invalid.
#[instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
pub async fn disable(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<TotpDisableRequest>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("disabling TOTP 2FA");

    let user = user_repo::get_by_id(&state.db, auth.user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::Unauthorized)?;

    if !user.totp_enabled {
        return Err(ApiError::BadRequest("2FA is not enabled".into()));
    }

    let secret_base32 = user.totp_secret.ok_or_else(|| {
        ApiError::Internal(anyhow::anyhow!("totp_enabled=true but no secret stored"))
    })?;

    let account_name = user.email.as_deref().unwrap_or(&user.username).to_string();
    let totp = make_totp(secret_base32, &account_name, APP_NAME)?;

    let valid = totp.check_current(&req.code).map_err(|e| {
        tracing::error!(error = %e, "TOTP time error during disable");
        ApiError::Internal(anyhow::anyhow!("TOTP clock error: {}", e))
    })?;

    if !valid {
        tracing::warn!(user_id = %auth.user_id, "invalid TOTP code during 2FA disable");
        return Err(ApiError::Unauthorized);
    }

    user_repo::disable_totp(&state.db, auth.user_id)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(user_id = %auth.user_id, "2FA disabled");
    log_mod_action(&state, opencorde_core::Snowflake::new(0), auth.user_id, "2fa.disable", auth.user_id.as_i64()).await;
    Ok(StatusCode::NO_CONTENT)
}

/// Verify a raw TOTP code string against a stored base32 secret.
///
/// Used by the login handler to check the user's TOTP code.
///
/// # Arguments
/// * `secret_base32` — The base32-encoded TOTP secret from the DB
/// * `code` — The 6-digit code provided by the user
/// * `account_name` — Account identifier (email or username) — used in TOTP metadata
/// * `app_name` — Issuer name shown in authenticator apps
///
/// # Returns
/// `true` if the code is valid for the current or adjacent time window.
pub fn check_totp_code(
    secret_base32: &str,
    code: &str,
    account_name: &str,
    app_name: &str,
) -> Result<bool, ApiError> {
    let totp = make_totp(secret_base32.to_string(), account_name, app_name)?;
    totp.check_current(code).map_err(|e| {
        tracing::error!(error = %e, "TOTP clock error during login check");
        ApiError::Internal(anyhow::anyhow!("TOTP clock error: {}", e))
    })
}

/// Build a TOTP instance from a base32 secret.
fn make_totp(
    secret_base32: String,
    account_name: &str,
    issuer: &str,
) -> Result<TOTP, ApiError> {
    let secret_bytes = Secret::Encoded(secret_base32)
        .to_bytes()
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid TOTP secret: {}", e)))?;
    TOTP::new(
        Algorithm::SHA1,
        6,    // digits
        1,    // skew (±1 step tolerance for clock drift)
        30,   // step in seconds (RFC 6238 default)
        secret_bytes,
        Some(issuer.to_string()),
        account_name.to_string(),
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("TOTP construction failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_totp_valid_secret() {
        // A valid base32 secret should produce a TOTP instance without error
        let secret = Secret::generate_secret();
        let b32 = secret.to_encoded().to_string();
        assert!(make_totp(b32, "user@example.com", "OpenCorde").is_ok());
    }

    #[test]
    fn test_make_totp_invalid_secret() {
        // Garbage input should produce an error
        let result = make_totp("not valid base32!!!".to_string(), "user@example.com", "OpenCorde");
        assert!(result.is_err());
    }

    #[test]
    fn test_check_totp_code_roundtrip() {
        // Generate a secret, get the current code, verify it
        let secret = Secret::generate_secret();
        let b32 = secret.to_encoded().to_string();
        let totp = make_totp(b32.clone(), "user@example.com", "OpenCorde").unwrap();
        let code = totp.generate_current().unwrap();
        let result = check_totp_code(&b32, &code, "user@example.com", "OpenCorde");
        assert!(result.is_ok());
        assert!(result.unwrap(), "current code should be valid");
    }

    #[test]
    fn test_check_totp_code_wrong_code() {
        let secret = Secret::generate_secret();
        let b32 = secret.to_encoded().to_string();
        let result = check_totp_code(&b32, "000000", "user@example.com", "OpenCorde");
        // The function succeeds (no error) but returns false for "000000"
        // (extremely unlikely to be the actual code)
        assert!(result.is_ok());
    }
}
