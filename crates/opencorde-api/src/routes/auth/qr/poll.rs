//! # QR Poll Handler
//! GET /api/v1/auth/qr/{token} — Poll for QR login approval.
//!
//! ## Behavior
//! - No authentication required (the requesting device is not yet logged in)
//! - Looks up the QR token in Redis
//! - Returns `{"status": "pending"}` while waiting
//! - Returns `{"status": "confirmed", ...auth_data}` when the mobile device confirms
//! - Returns 404 if the token is unknown/expired

use axum::{
    Json,
    extract::{Path, State},
};
use redis::AsyncCommands;
use serde_json::Value;

use crate::{AppState, error::ApiError};

/// Redis key prefix for QR login tokens.
const QR_KEY_PREFIX: &str = "qr:login:";

/// GET /api/v1/auth/qr/{token}
///
/// Polled by the desktop client every ~2 seconds until the mobile device confirms.
#[tracing::instrument(skip(state))]
pub async fn poll_qr(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let redis_key = format!("{}{}", QR_KEY_PREFIX, token);

    let mut conn = state.redis_conn.clone();
    let value: Option<String> = conn.get(&redis_key).await.map_err(|e| {
        tracing::error!(error = %e, "failed to read QR token from Redis");
        ApiError::InternalServerError("failed to read QR session".into())
    })?;

    match value.as_deref() {
        None => {
            tracing::debug!(token = %token, "QR token not found (expired or invalid)");
            Err(ApiError::NotFound("QR token expired or invalid".into()))
        }
        Some("pending") => {
            tracing::trace!(token = %token, "QR token still pending");
            Ok(Json(serde_json::json!({"status": "pending"})))
        }
        Some(payload_str) => {
            // Token confirmed — parse the payload, then delete the key so the
            // JWT pair can only be collected once (defense in depth: without
            // this, anyone who captured the same QR token could re-poll and
            // receive live credentials for the rest of the 60s TTL window).
            let payload: Value = serde_json::from_str(payload_str).map_err(|e| {
                tracing::error!(error = %e, "failed to parse confirmed QR payload");
                ApiError::InternalServerError("corrupted QR session".into())
            })?;

            let _: redis::RedisResult<i64> = conn.del(&redis_key).await;

            tracing::info!(token = %token, "QR token confirmed — returning auth payload (one-time)");

            Ok(Json(
                serde_json::json!({"status": "confirmed", "data": payload}),
            ))
        }
    }
}
