//! # E2EE Key Package Endpoints
//!
//! ## Endpoints
//! - POST   /api/v1/users/me/key-packages            — Upload a new KeyPackage (base64 body)
//! - GET    /api/v1/users/{user_id}/key-packages/one — Consume one available KeyPackage
//! - DELETE /api/v1/users/me/key-packages            — Delete all own KeyPackages
//!
//! ## Depends On
//! - axum (web framework)
//! - crate::middleware::auth::AuthUser (authentication)
//! - crate::AppState (application state)

use crate::{error::ApiError, middleware::auth::AuthUser, routes::helpers, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tracing::instrument;

/// Request body for uploading a KeyPackage.
#[derive(Debug, Deserialize)]
pub struct UploadKeyPackageRequest {
    /// Base64-encoded TLS-serialized OpenMLS KeyPackage
    pub key_package: String,
}

/// Response when fetching a KeyPackage for another user.
#[derive(Debug, Serialize)]
pub struct KeyPackageResponse {
    /// Key package database ID (for reference)
    pub id: i64,
    /// Base64-encoded TLS-serialized OpenMLS KeyPackage
    pub key_package: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/users/me/key-packages",
            post(upload_key_package).delete(delete_all_key_packages),
        )
        .route(
            "/api/v1/users/{user_id}/key-packages/one",
            get(consume_key_package),
        )
}

/// POST /api/v1/users/me/key-packages — Upload a new KeyPackage.
///
/// Clients should upload several key packages at a time (typically 10-20).
/// Each package is single-use: consumed when the user is added to a group.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn upload_key_package(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<UploadKeyPackageRequest>,
) -> Result<StatusCode, ApiError> {
    // Decode base64 → raw bytes
    let kp_bytes = base64_decode(&payload.key_package)?;

    sqlx::query(
        "INSERT INTO e2ee_key_packages (user_id, key_package) VALUES ($1, $2)",
    )
    .bind(auth.user_id.as_i64())
    .bind(&kp_bytes)
    .execute(&state.db)
    .await
    .map_err(ApiError::Database)?;

    tracing::info!(user_id = %auth.user_id, "key package uploaded");
    Ok(StatusCode::CREATED)
}

/// GET /api/v1/users/{user_id}/key-packages/one — Consume one KeyPackage for a user.
///
/// Marks the package as consumed (sets consumed_at) so it is never returned again.
/// Returns 404 if no packages remain — caller must request the user upload more.
#[instrument(skip(state, auth), fields(requester = %auth.user_id))]
pub async fn consume_key_package(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id_str): Path<String>,
) -> Result<Json<KeyPackageResponse>, ApiError> {
    let target_id = helpers::parse_snowflake(&user_id_str)?;

    // Atomically select and mark one available package as consumed
    let row = sqlx::query(
        r#"
        UPDATE e2ee_key_packages
        SET consumed_at = NOW()
        WHERE id = (
            SELECT id FROM e2ee_key_packages
            WHERE user_id = $1 AND consumed_at IS NULL
            ORDER BY created_at ASC
            LIMIT 1
            FOR UPDATE SKIP LOCKED
        )
        RETURNING id, key_package
        "#,
    )
    .bind(target_id.as_i64())
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("no key packages available for this user".into()))?;

    let kp_id: i64 = row.get("id");
    let kp_bytes: Vec<u8> = row.get("key_package");

    tracing::info!(
        requester = %auth.user_id,
        target_user = target_id.as_i64(),
        kp_id = kp_id,
        "key package consumed"
    );

    Ok(Json(KeyPackageResponse {
        id: kp_id,
        key_package: base64_encode(&kp_bytes),
    }))
}

/// DELETE /api/v1/users/me/key-packages — Delete all own key packages.
///
/// Called on logout or when rotating keys. Deletes all packages (consumed or not).
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn delete_all_key_packages(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<StatusCode, ApiError> {
    let deleted = sqlx::query(
        "DELETE FROM e2ee_key_packages WHERE user_id = $1",
    )
    .bind(auth.user_id.as_i64())
    .execute(&state.db)
    .await
    .map_err(ApiError::Database)?
    .rows_affected();

    tracing::info!(user_id = %auth.user_id, deleted, "key packages deleted");
    Ok(StatusCode::NO_CONTENT)
}

/// Encode bytes to base64 (URL-safe, no padding).
fn base64_encode(bytes: &[u8]) -> String {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Decode base64 to bytes (URL-safe, no padding).
fn base64_decode(s: &str) -> Result<Vec<u8>, ApiError> {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|_| ApiError::BadRequest("invalid base64 in key_package".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_roundtrip() {
        let original = b"hello MLS key package";
        let encoded = base64_encode(original);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_base64_invalid() {
        assert!(base64_decode("not!valid!base64!!!").is_err());
    }
}
