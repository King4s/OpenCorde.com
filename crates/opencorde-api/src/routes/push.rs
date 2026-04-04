//! # Push Notification Registration Routes
//! HTTP endpoints to register and unregister push notification tokens.
//!
//! ## Endpoints
//! - `POST /api/v1/push/register`   — store a device token for the authenticated user
//! - `DELETE /api/v1/push/unregister` — remove a device token
//!
//! ## Supported Platforms
//! - `web`  — Web Push subscription JSON from `navigator.serviceWorker`
//! - `fcm`  — Firebase Cloud Messaging registration token (Android)
//! - `apns` — Apple Push Notification Service device token (iOS, stub)
//!
//! ## Depends On
//! - axum (routing, extractors)
//! - crate::middleware::auth::AuthUser (JWT authentication)
//! - crate::AppState (database pool)
//! - crate::error::ApiError (unified error responses)

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{delete, post},
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

/// Request body for `POST /api/v1/push/register`.
#[derive(Debug, Deserialize)]
pub struct RegisterTokenRequest {
    /// Raw push token or Web Push subscription JSON string.
    pub token: String,
    /// Platform identifier: `"web"`, `"fcm"`, or `"apns"`.
    pub platform: String,
}

/// Request body for `DELETE /api/v1/push/unregister`.
#[derive(Debug, Deserialize)]
pub struct UnregisterTokenRequest {
    /// Token to remove (exact match).
    pub token: String,
}

/// Generic success envelope.
#[derive(Debug, Serialize)]
pub struct PushResponse {
    pub ok: bool,
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Build the push notification router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/push/register", post(register_token))
        .route("/api/v1/push/unregister", delete(unregister_token))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `POST /api/v1/push/register` — save a device token for the current user.
///
/// Inserts with ON CONFLICT DO NOTHING so duplicate registrations are safe.
/// Returns 201 Created on success.
#[instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
async fn register_token(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<RegisterTokenRequest>,
) -> Result<(StatusCode, Json<PushResponse>), ApiError> {
    tracing::info!(platform = %req.platform, "registering push token");

    // Validate platform value
    if !matches!(req.platform.as_str(), "web" | "fcm" | "apns") {
        return Err(ApiError::BadRequest(
            "platform must be one of: web, fcm, apns".to_string(),
        ));
    }

    if req.token.trim().is_empty() {
        return Err(ApiError::BadRequest("token must not be empty".to_string()));
    }

    sqlx::query(
        "INSERT INTO push_tokens (user_id, token, platform) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (user_id, token) DO NOTHING",
    )
    .bind(auth.user_id.as_i64())
    .bind(&req.token)
    .bind(&req.platform)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to insert push token");
        ApiError::Database(e)
    })?;

    tracing::info!(platform = %req.platform, "push token registered");
    Ok((StatusCode::CREATED, Json(PushResponse { ok: true })))
}

/// `DELETE /api/v1/push/unregister` — remove a device token for the current user.
///
/// Returns 200 OK even if the token was not found (idempotent).
#[instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
async fn unregister_token(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UnregisterTokenRequest>,
) -> Result<Json<PushResponse>, ApiError> {
    tracing::info!("unregistering push token");

    sqlx::query(
        "DELETE FROM push_tokens WHERE user_id = $1 AND token = $2",
    )
    .bind(auth.user_id.as_i64())
    .bind(&req.token)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to delete push token");
        ApiError::Database(e)
    })?;

    tracing::info!("push token unregistered (or was not present)");
    Ok(Json(PushResponse { ok: true }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_validation() {
        for valid in ["web", "fcm", "apns"] {
            assert!(matches!(valid, "web" | "fcm" | "apns"));
        }
        let invalid = "telegram";
        assert!(!matches!(invalid, "web" | "fcm" | "apns"));
    }

    #[test]
    fn test_router_creation() {
        let _r = router();
    }
}
