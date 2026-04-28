//! # E2EE Group Endpoints
//!
//! ## Endpoints
//! - POST /api/v1/channels/{channel_id}/e2ee/init    — Store initial group state + welcome messages
//! - GET  /api/v1/channels/{channel_id}/e2ee/welcome — Fetch + consume own welcome message
//! - PUT  /api/v1/channels/{channel_id}/e2ee/state   — Update own group state after processing commit
//!
//! ## Flow
//! 1. Channel creator POSTs init: provides their own group_state + array of
//!    {user_id, welcome_message} for each existing member.
//! 2. Each member GETs /welcome to receive their Welcome bytes (cleared after fetch).
//! 3. On every MLS commit, the committing member PUTs /state with updated group_state.
//!
//! ## Depends On
//! - axum (web framework)
//! - crate::middleware::auth::AuthUser (authentication)
//! - crate::AppState (application state)

use crate::{
    AppState,
    error::ApiError,
    middleware::auth::AuthUser,
    routes::{helpers, permission_check},
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
};
use opencorde_core::permissions::Permissions;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tracing::instrument;

/// Welcome entry for a single member during group init.
#[derive(Debug, Deserialize)]
pub struct MemberWelcome {
    /// Member's user ID (string Snowflake)
    pub user_id: String,
    /// Base64-encoded MLS Welcome message for this member
    pub welcome_message: String,
}

/// Request body for POST /e2ee/init.
#[derive(Debug, Deserialize)]
pub struct InitGroupRequest {
    /// Base64-encoded MLS group state for the creator
    pub group_state: String,
    /// Welcome messages for each member being added (may be empty for solo channels)
    pub member_welcomes: Vec<MemberWelcome>,
}

/// Response for GET /e2ee/welcome.
#[derive(Debug, Serialize)]
pub struct WelcomeResponse {
    /// Base64-encoded MLS Welcome bytes
    pub welcome_message: String,
}

/// Request body for PUT /e2ee/state.
#[derive(Debug, Deserialize)]
pub struct UpdateStateRequest {
    /// Base64-encoded updated MLS group state
    pub group_state: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/channels/{channel_id}/e2ee/init", post(init_group))
        .route(
            "/api/v1/channels/{channel_id}/e2ee/welcome",
            get(get_welcome),
        )
        .route(
            "/api/v1/channels/{channel_id}/e2ee/state",
            put(update_state),
        )
}

/// POST /api/v1/channels/{channel_id}/e2ee/init — Initialize E2EE group.
///
/// Called once by the group creator. Stores creator's group state and
/// welcome messages for all initial members.
#[instrument(skip(state, auth, payload), fields(user_id = %auth.user_id))]
pub async fn init_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id_str): Path<String>,
    Json(payload): Json<InitGroupRequest>,
) -> Result<StatusCode, ApiError> {
    let channel_id = helpers::parse_snowflake(&channel_id_str)?;
    let creator_state = base64_decode(&payload.group_state)?;

    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id,
        Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
    )
    .await?;

    // Upsert creator's own group state (no welcome needed for creator)
    sqlx::query(
        r#"
        INSERT INTO e2ee_groups (channel_id, user_id, group_state)
        VALUES ($1, $2, $3)
        ON CONFLICT (channel_id, user_id)
        DO UPDATE SET group_state = EXCLUDED.group_state, updated_at = NOW()
        "#,
    )
    .bind(channel_id.as_i64())
    .bind(auth.user_id.as_i64())
    .bind(&creator_state)
    .execute(&state.db)
    .await
    .map_err(ApiError::Database)?;

    // Insert welcome messages for each member
    for member in &payload.member_welcomes {
        let member_id = helpers::parse_snowflake(&member.user_id)?;
        let welcome_bytes = base64_decode(&member.welcome_message)?;

        sqlx::query(
            r#"
            INSERT INTO e2ee_groups (channel_id, user_id, group_state, welcome_message)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (channel_id, user_id)
            DO UPDATE SET welcome_message = EXCLUDED.welcome_message, updated_at = NOW()
            "#,
        )
        .bind(channel_id.as_i64())
        .bind(member_id.as_i64())
        .bind(&creator_state) // members receive current epoch state as placeholder
        .bind(&welcome_bytes)
        .execute(&state.db)
        .await
        .map_err(ApiError::Database)?;
    }

    tracing::info!(
        user_id = %auth.user_id,
        channel_id = channel_id.as_i64(),
        member_count = payload.member_welcomes.len(),
        "E2EE group initialized"
    );
    Ok(StatusCode::CREATED)
}

/// GET /api/v1/channels/{channel_id}/e2ee/welcome — Fetch and consume own welcome message.
///
/// Returns the Welcome message once, then clears it from the DB.
/// Returns 404 if no welcome is pending (already consumed or not invited).
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn get_welcome(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id_str): Path<String>,
) -> Result<Json<WelcomeResponse>, ApiError> {
    let channel_id = helpers::parse_snowflake(&channel_id_str)?;

    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id,
        Permissions::VIEW_CHANNEL,
    )
    .await?;

    // Atomically fetch and clear the welcome message
    let row = sqlx::query(
        r#"
        UPDATE e2ee_groups
        SET welcome_message = NULL, updated_at = NOW()
        WHERE channel_id = $1 AND user_id = $2 AND welcome_message IS NOT NULL
        RETURNING welcome_message
        "#,
    )
    .bind(channel_id.as_i64())
    .bind(auth.user_id.as_i64())
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("no pending welcome message for this channel".into()))?;

    let welcome_bytes: Option<Vec<u8>> = row.get("welcome_message");
    let welcome_bytes = welcome_bytes
        .ok_or_else(|| ApiError::NotFound("welcome message already consumed".into()))?;

    tracing::info!(
        user_id = %auth.user_id,
        channel_id = channel_id.as_i64(),
        "welcome message consumed"
    );

    Ok(Json(WelcomeResponse {
        welcome_message: base64_encode(&welcome_bytes),
    }))
}

/// PUT /api/v1/channels/{channel_id}/e2ee/state — Update own MLS group state.
///
/// Called after processing any MLS commit. Replaces the stored group_state blob.
#[instrument(skip(state, auth, payload), fields(user_id = %auth.user_id))]
pub async fn update_state(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id_str): Path<String>,
    Json(payload): Json<UpdateStateRequest>,
) -> Result<StatusCode, ApiError> {
    let channel_id = helpers::parse_snowflake(&channel_id_str)?;
    let new_state = base64_decode(&payload.group_state)?;

    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id,
        Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
    )
    .await?;

    let result = sqlx::query(
        r#"
        UPDATE e2ee_groups
        SET group_state = $1, updated_at = NOW()
        WHERE channel_id = $2 AND user_id = $3
        "#,
    )
    .bind(&new_state)
    .bind(channel_id.as_i64())
    .bind(auth.user_id.as_i64())
    .execute(&state.db)
    .await
    .map_err(ApiError::Database)?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(
            "no E2EE group found for this channel; call /init first".into(),
        ));
    }

    tracing::debug!(
        user_id = %auth.user_id,
        channel_id = channel_id.as_i64(),
        "group state updated"
    );
    Ok(StatusCode::NO_CONTENT)
}

fn base64_encode(bytes: &[u8]) -> String {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.encode(bytes)
}

fn base64_decode(s: &str) -> Result<Vec<u8>, ApiError> {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|_| ApiError::BadRequest("invalid base64".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_roundtrip() {
        let data = b"MLS welcome message bytes";
        assert_eq!(base64_decode(&base64_encode(data)).unwrap(), data);
    }
}
