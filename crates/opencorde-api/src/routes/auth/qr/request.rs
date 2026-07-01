//! # QR Request Handler
//! POST /api/v1/auth/qr/request — Generate a short-lived QR login token.
//!
//! ## Behavior
//! - Generates a UUID v4 token
//! - Stores "pending" in Redis with 5-minute TTL
//! - Returns token + expiry to the requesting (unauthenticated) client

use axum::{Json, extract::State};
use redis::AsyncCommands;
use serde::Serialize;
use uuid::Uuid;

use crate::{AppState, error::ApiError};

/// QR token expiry in seconds (5 minutes).
const QR_TOKEN_TTL_SECS: u64 = 300;

/// Redis key prefix for QR login tokens.
const QR_KEY_PREFIX: &str = "qr:login:";

/// Response for a successful QR request.
#[derive(Debug, Serialize)]
pub struct QrRequestResponse {
    /// The QR login token (UUID v4).
    pub token: String,
    /// Expiry timestamp as Unix seconds.
    pub expires_at: i64,
}

/// POST /api/v1/auth/qr/request
///
/// Creates a new QR login session token. No authentication required —
/// this is the endpoint the desktop client calls to initiate device handoff.
#[tracing::instrument(skip(state))]
pub async fn request_qr(
    State(state): State<AppState>,
) -> Result<Json<QrRequestResponse>, ApiError> {
    tracing::info!("QR login request initiated");

    let token = Uuid::new_v4().to_string();
    let redis_key = format!("{}{}", QR_KEY_PREFIX, token);

    // Store "pending" in Redis with 5-min TTL
    let mut conn = state.redis_conn.clone();
    let _: () = conn
        .set_ex(&redis_key, "pending", QR_TOKEN_TTL_SECS)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to store QR token in Redis");
            ApiError::InternalServerError("failed to create QR session".into())
        })?;

    let expires_at = chrono::Utc::now().timestamp() + QR_TOKEN_TTL_SECS as i64;

    tracing::info!(token = %token, expires_at, "QR login token created");

    Ok(Json(QrRequestResponse {
        token,
        expires_at,
    }))
}
