//! # Route: Server Onboarding
//! Configuration for the server onboarding flow shown to new members.
//!
//! ## Endpoints
//! - GET /api/v1/servers/{server_id}/onboarding       — get config
//! - PUT /api/v1/servers/{server_id}/onboarding       — upsert config (owner only)
//!
//! ## Depends On
//! - sqlx, AppState, ApiError, AuthUser

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser, routes::permission_check};
use crate::routes::helpers::parse_snowflake;
use opencorde_core::permissions::Permissions;
use opencorde_db::repos::server_repo;

#[derive(Debug, Serialize)]
pub struct OnboardingResponse {
    pub server_id: String,
    pub enabled: bool,
    pub welcome_message: Option<String>,
    pub prompts: Value,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOnboardingRequest {
    pub enabled: Option<bool>,
    pub welcome_message: Option<String>,
    pub prompts: Option<Value>,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/api/v1/servers/{server_id}/onboarding",
        get(get_onboarding).put(update_onboarding),
    )
}

#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn get_onboarding(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
) -> Result<Json<OnboardingResponse>, ApiError> {
    let sid = parse_snowflake(&server_id)?;
    permission_check::require_server_perm(&state.db, auth.user_id, sid, Permissions::VIEW_CHANNEL).await?;

    let row: Option<(bool, Option<String>, Value, DateTime<Utc>)> = sqlx::query_as(
        "SELECT enabled, welcome_message, prompts, updated_at \
         FROM server_onboarding WHERE server_id = $1",
    )
    .bind(sid.as_i64())
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    match row {
        Some((enabled, welcome_message, prompts, updated_at)) => Ok(Json(OnboardingResponse {
            server_id: sid.as_i64().to_string(),
            enabled,
            welcome_message,
            prompts,
            updated_at,
        })),
        None => Ok(Json(OnboardingResponse {
            server_id: sid.as_i64().to_string(),
            enabled: false,
            welcome_message: None,
            prompts: serde_json::json!([]),
            updated_at: Utc::now(),
        })),
    }
}

#[tracing::instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
async fn update_onboarding(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
    Json(req): Json<UpdateOnboardingRequest>,
) -> Result<Json<OnboardingResponse>, ApiError> {
    let sid = parse_snowflake(&server_id)?;
    let _server = server_repo::get_by_id(&state.db, sid)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;
    permission_check::require_server_perm(&state.db, auth.user_id, sid, Permissions::MANAGE_SERVER).await?;

    let enabled = req.enabled.unwrap_or(false);
    let prompts = req.prompts.unwrap_or_else(|| serde_json::json!([]));

    let row: (bool, Option<String>, Value, DateTime<Utc>) = sqlx::query_as(
        "INSERT INTO server_onboarding (server_id, enabled, welcome_message, prompts, updated_at)
         VALUES ($1, $2, $3, $4, NOW())
         ON CONFLICT (server_id) DO UPDATE SET
           enabled = EXCLUDED.enabled,
           welcome_message = EXCLUDED.welcome_message,
           prompts = EXCLUDED.prompts,
           updated_at = NOW()
         RETURNING enabled, welcome_message, prompts, updated_at",
    )
    .bind(sid.as_i64())
    .bind(enabled)
    .bind(&req.welcome_message)
    .bind(&prompts)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    tracing::info!(server_id = sid.as_i64(), enabled, "onboarding config updated");

    Ok(Json(OnboardingResponse {
        server_id: sid.as_i64().to_string(),
        enabled: row.0,
        welcome_message: row.1,
        prompts: row.2,
        updated_at: row.3,
    }))
}
