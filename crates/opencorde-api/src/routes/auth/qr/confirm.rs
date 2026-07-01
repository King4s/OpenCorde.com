//! # QR Confirm Handler
//! POST /api/v1/auth/qr/confirm — Authenticated user confirms a QR login request.
//!
//! ## Behavior
//! - Requires authentication (the confirming device is already logged in)
//! - Looks up the QR token in Redis
//! - If token is "pending", generates a JWT for the requesting device
//! - Stores the serialized AuthResponse JSON in Redis under the token key
//! - Resets TTL to 60 seconds (short window for the poller to collect)

use axum::{Json, extract::State};
use redis::AsyncCommands;
use serde::Deserialize;

use crate::{
    AppState,
    error::ApiError,
    jwt,
    middleware::auth::AuthUser,
};

/// Short TTL after confirmation (60s — enough for the poller to pick up).
const CONFIRMED_TTL_SECS: u64 = 60;

/// Redis key prefix for QR login tokens.
const QR_KEY_PREFIX: &str = "qr:login:";

/// Request body for QR confirmation.
#[derive(Debug, Deserialize)]
pub struct QrConfirmRequest {
    /// The QR token to confirm.
    pub token: String,
}

/// POST /api/v1/auth/qr/confirm
///
/// The already-authenticated mobile device calls this after scanning a QR code.
/// On success, the requesting (desktop) device's next poll will receive a JWT.
#[tracing::instrument(skip(state, auth_user, req))]
pub async fn confirm_qr(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(req): Json<QrConfirmRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    tracing::info!(
        user_id = %auth_user.user_id,
        token = %req.token,
        "QR login confirmation attempt"
    );

    let redis_key = format!("{}{}", QR_KEY_PREFIX, req.token);

    // Check token exists and is still pending
    let mut conn = state.redis_conn.clone();
    let status: Option<String> = conn.get(&redis_key).await.map_err(|e| {
        tracing::error!(error = %e, "failed to read QR token from Redis");
        ApiError::InternalServerError("failed to read QR session".into())
    })?;

    match status.as_deref() {
        None => {
            tracing::warn!(token = %req.token, "QR token not found or expired");
            return Err(ApiError::NotFound("QR token expired or invalid".into()));
        }
        Some("pending") => {
            // Good — proceed
        }
        Some(_) => {
            tracing::warn!(token = %req.token, "QR token already confirmed");
            return Err(ApiError::Conflict("QR token already used".into()));
        }
    }

    // Generate JWT tokens for the requesting device
    // No refresh token yet — the desktop will get access_token + set refresh cookie
    let (refresh_token, jti) = jwt::create_refresh_token(
        auth_user.user_id,
        &auth_user.username,
        &state.config.jwt_secret,
        state.config.jwt_refresh_expiry,
        None,
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("token creation failed: {}", e)))?;

    let access_token = jwt::create_access_token(
        auth_user.user_id,
        &auth_user.username,
        &state.config.jwt_secret,
        state.config.jwt_access_expiry,
        None,
        Some(&jti),
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("token creation failed: {}", e)))?;

    // Store the JTI in DB for rotation/theft detection
    let expires_at =
        chrono::Utc::now() + chrono::Duration::seconds(state.config.jwt_refresh_expiry as i64);
    opencorde_db::repos::refresh_token_repo::insert(
        &state.db,
        &jti,
        auth_user.user_id.as_i64(),
        expires_at,
    )
    .await
    .map_err(|e| {
        ApiError::Internal(anyhow::anyhow!(
            "failed to store refresh token JTI: {}",
            e
        ))
    })?;

    // Build the auth payload to deliver to the polling device
    let payload = serde_json::json!({
        "access_token": access_token,
        "refresh_token": refresh_token,
        "session_jti": jti,
        "expires_in": state.config.jwt_access_expiry,
        "user": {
            "id": auth_user.user_id.as_i64().to_string(),
            "username": auth_user.username,
        },
    });

    let payload_str = serde_json::to_string(&payload).map_err(|e| {
        ApiError::Internal(anyhow::anyhow!("failed to serialize auth payload: {}", e))
    })?;

    // Replace "pending" with the JWT payload, shorter TTL
    let _: () = conn
        .set_ex(&redis_key, &payload_str, CONFIRMED_TTL_SECS)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to store confirmed QR token");
            ApiError::InternalServerError("failed to confirm QR session".into())
        })?;

    tracing::info!(
        user_id = %auth_user.user_id,
        token = %req.token,
        "QR login confirmed — JWT stored for polling device"
    );

    Ok(Json(serde_json::json!({"status": "confirmed"})))
}
