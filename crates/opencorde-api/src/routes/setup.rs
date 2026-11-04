//! # Route: Instance Setup
//! Instance-wide setup wizard for first-time configuration.
//!
//! ## Endpoints
//! - GET /api/v1/setup/status       — check if setup is complete (public)
//! - POST /api/v1/setup/complete    — finish setup: create admin + mark done (public, once)
//!
//! ## Depends On
//! - sqlx, AppState, ApiError

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{AppState, error::ApiError};
use opencorde_core::{SnowflakeGenerator, generate_keypair, password};
use opencorde_db::repos::user_repo;

#[derive(Debug, Serialize)]
pub struct SetupStatusResponse {
    pub setup_completed: bool,
    pub instance_name: Option<String>,
    pub base_url: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CompleteSetupRequest {
    pub instance_name: String,
    pub base_url: String,
    pub admin_username: String,
    pub admin_email: String,
    pub admin_password: String,
}

#[derive(Debug, Serialize)]
pub struct CompleteSetupResponse {
    pub setup_completed: bool,
    pub instance_name: String,
    pub base_url: String,
    pub admin_user_id: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/setup/status", get(get_setup_status))
        .route("/api/v1/setup/complete", post(complete_setup))
}

/// GET /api/v1/setup/status — Check whether the instance has been set up.
///
/// Public endpoint; used by the frontend to decide whether to show the setup wizard.
#[tracing::instrument(skip(state))]
async fn get_setup_status(
    State(state): State<AppState>,
) -> Result<Json<SetupStatusResponse>, ApiError> {
    let row: Option<(bool, Option<String>, Option<String>, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT setup_completed, instance_name, base_url, completed_at FROM instance_setup WHERE id = TRUE",
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    match row {
        Some((setup_completed, instance_name, base_url, completed_at)) => {
            Ok(Json(SetupStatusResponse {
                setup_completed,
                instance_name,
                base_url,
                completed_at,
            }))
        }
        None => {
            // No row yet — setup not started. Also check if any users exist as a fallback.
            let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
                .fetch_one(&state.db)
                .await
                .map_err(|e| ApiError::Internal(e.into()))?;

            let setup_completed = user_count.0 > 0;

            Ok(Json(SetupStatusResponse {
                setup_completed,
                instance_name: None,
                base_url: None,
                completed_at: None,
            }))
        }
    }
}

/// POST /api/v1/setup/complete — Finish instance setup.
///
/// Creates the first admin user, records instance metadata, and marks setup as complete.
/// This endpoint only works once — after setup is complete it returns 409 Conflict.
#[tracing::instrument(skip(state, req))]
async fn complete_setup(
    State(state): State<AppState>,
    Json(req): Json<CompleteSetupRequest>,
) -> Result<(StatusCode, Json<CompleteSetupResponse>), ApiError> {
    tracing::info!(instance_name = %req.instance_name, admin_username = %req.admin_username, "setup completion attempt");

    // Validate inputs
    if req.instance_name.len() < 1 || req.instance_name.len() > 128 {
        return Err(ApiError::BadRequest(
            "instance_name must be between 1 and 128 characters".into(),
        ));
    }
    if req.base_url.len() < 1 || req.base_url.len() > 256 {
        return Err(ApiError::BadRequest(
            "base_url must be between 1 and 256 characters".into(),
        ));
    }
    if req.admin_username.len() < 2 || req.admin_username.len() > 32 {
        return Err(ApiError::BadRequest(
            "admin_username must be between 2 and 32 characters".into(),
        ));
    }
    if req.admin_password.len() < 8 {
        return Err(ApiError::BadRequest(
            "admin_password must be at least 8 characters".into(),
        ));
    }

    // Check if setup is already complete
    let existing: Option<(bool,)> =
        sqlx::query_as("SELECT setup_completed FROM instance_setup WHERE id = TRUE")
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

    if let Some((true,)) = existing {
        tracing::warn!("setup already completed");
        return Err(ApiError::Conflict("setup already completed".into()));
    }

    // Also guard against users already existing (fallback safety)
    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    if user_count.0 > 0 {
        tracing::warn!("users already exist; blocking setup");
        return Err(ApiError::Conflict(
            "users already exist; setup cannot be completed".into(),
        ));
    }

    // Generate Ed25519 keypair for admin identity
    let (_private_key, public_key) = generate_keypair();
    tracing::debug!("Ed25519 keypair generated for admin");

    // Hash admin password
    let password_hash = password::hash_password(&req.admin_password)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("password hashing failed: {}", e)))?;

    // Generate Snowflake ID for admin user
    let mut generator = SnowflakeGenerator::new(0, 0);
    let admin_user_id = generator.next_id();

    // Create admin user
    let user_row = user_repo::create_user(
        &state.db,
        admin_user_id,
        &req.admin_username,
        &public_key,
        Some(&req.admin_email),
        Some(&password_hash),
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to create admin user");
        ApiError::Database(e)
    })?;

    tracing::info!(user_id = user_row.id, "admin user created");

    // Mark email as verified for admin
    let _ = sqlx::query("UPDATE users SET email_verified = TRUE WHERE id = $1")
        .bind(user_row.id)
        .execute(&state.db)
        .await;

    // Insert or update instance_setup row
    let _ = sqlx::query(
        "INSERT INTO instance_setup (id, setup_completed, instance_name, base_url, admin_user_id, completed_at, created_at)
         VALUES (TRUE, TRUE, $1, $2, $3, NOW(), NOW())
         ON CONFLICT (id) DO UPDATE SET
           setup_completed = TRUE,
           instance_name = EXCLUDED.instance_name,
           base_url = EXCLUDED.base_url,
           admin_user_id = EXCLUDED.admin_user_id,
           completed_at = NOW()",
    )
    .bind(&req.instance_name)
    .bind(&req.base_url)
    .bind(user_row.id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    tracing::info!("instance setup completed successfully");

    Ok((
        StatusCode::CREATED,
        Json(CompleteSetupResponse {
            setup_completed: true,
            instance_name: req.instance_name,
            base_url: req.base_url,
            admin_user_id: admin_user_id.to_string(),
        }),
    ))
}
