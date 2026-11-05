//! # E2EE Group Endpoints
//!
//! ## Endpoints
//! - POST /api/v1/channels/{channel_id}/e2ee/init    — Relay encrypted welcome messages
//! - GET  /api/v1/channels/{channel_id}/e2ee/welcome — Fetch + consume own welcome message
//! - PUT  /api/v1/channels/{channel_id}/e2ee/state   — Legacy empty state acknowledgement
//!
//! ## Flow
//! 1. Channel creator POSTs init: provides an array of
//!    {user_id, welcome_message} for each existing member.
//! 2. Each member GETs /welcome to receive their Welcome bytes (cleared after fetch).
//! 3. Private MLS state remains on the client. Nonempty state uploads are rejected.
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
use opencorde_core::{permissions::Permissions, snowflake::Snowflake};
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
    /// Legacy field: only an omitted or empty value is accepted. Private state stays local.
    #[serde(default)]
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
    /// Legacy field: nonempty state uploads are rejected; empty requests are a no-op.
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
/// Called by the group creator to relay encrypted Welcome messages.
/// Private group state is never uploaded; historical stored blobs are preserved.
#[instrument(skip(state, auth, payload), fields(user_id = %auth.user_id))]
pub async fn init_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id_str): Path<String>,
    Json(payload): Json<InitGroupRequest>,
) -> Result<StatusCode, ApiError> {
    let channel_id = helpers::parse_snowflake(&channel_id_str)?;
    init_group_for_user(&state.db, auth.user_id, channel_id, payload).await
}

async fn init_group_for_user(
    pool: &sqlx::PgPool,
    user_id: Snowflake,
    channel_id: Snowflake,
    payload: InitGroupRequest,
) -> Result<StatusCode, ApiError> {
    reject_private_state(&payload.group_state)?;
    super::sync::reject_legacy(pool, channel_id).await?;

    permission_check::require_channel_perm(
        pool,
        user_id,
        channel_id,
        Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
    )
    .await?;

    let mut member_welcomes = Vec::with_capacity(payload.member_welcomes.len());
    for member in &payload.member_welcomes {
        let member_id = helpers::parse_snowflake(&member.user_id)?;
        permission_check::require_channel_perm(
            pool,
            member_id,
            channel_id,
            Permissions::VIEW_CHANNEL,
        )
        .await
        .map_err(|_| {
            tracing::warn!(
                creator = user_id.as_i64(),
                member_id = member_id.as_i64(),
                channel_id = channel_id.as_i64(),
                "E2EE group init denied: welcome target cannot view channel"
            );
            ApiError::Forbidden
        })?;
        member_welcomes.push((member_id, base64_decode(&member.welcome_message)?));
    }

    // Record a new creator without private state; never overwrite historical blobs.
    sqlx::query(
        r#"
        INSERT INTO e2ee_groups (channel_id, user_id, group_state)
        VALUES ($1, $2, $3)
        ON CONFLICT (channel_id, user_id)
        DO NOTHING
        "#,
    )
    .bind(channel_id.as_i64())
    .bind(user_id.as_i64())
    .bind(Vec::<u8>::new())
    .execute(pool)
    .await
    .map_err(ApiError::Database)?;

    // Insert welcome messages for each member
    for (member_id, welcome_bytes) in &member_welcomes {
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
        .bind(Vec::<u8>::new()) // private MLS state is never stored
        .bind(welcome_bytes)
        .execute(pool)
        .await
        .map_err(ApiError::Database)?;
    }

    tracing::info!(
        user_id = %user_id,
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
    get_welcome_for_user(&state.db, auth.user_id, channel_id).await
}

async fn get_welcome_for_user(
    pool: &sqlx::PgPool,
    user_id: Snowflake,
    channel_id: Snowflake,
) -> Result<Json<WelcomeResponse>, ApiError> {
    permission_check::require_channel_perm(pool, user_id, channel_id, Permissions::VIEW_CHANNEL)
        .await?;

    // Atomically fetch and clear the welcome message
    super::sync::reject_legacy(pool, channel_id).await?;
    let row = sqlx::query(
        r#"
        WITH pending AS (
            SELECT channel_id, user_id, welcome_message
            FROM e2ee_groups
            WHERE channel_id = $1 AND user_id = $2 AND welcome_message IS NOT NULL
            FOR UPDATE
        )
        UPDATE e2ee_groups AS groups
        SET welcome_message = NULL, updated_at = NOW()
        FROM pending
        WHERE groups.channel_id = pending.channel_id AND groups.user_id = pending.user_id
        RETURNING pending.welcome_message
        "#,
    )
    .bind(channel_id.as_i64())
    .bind(user_id.as_i64())
    .fetch_optional(pool)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("no pending welcome message for this channel".into()))?;

    let welcome_bytes: Option<Vec<u8>> = row.get("welcome_message");
    let welcome_bytes = welcome_bytes
        .ok_or_else(|| ApiError::NotFound("welcome message already consumed".into()))?;

    tracing::info!(
        user_id = %user_id,
        channel_id = channel_id.as_i64(),
        "welcome message consumed"
    );

    Ok(Json(WelcomeResponse {
        welcome_message: base64_encode(&welcome_bytes),
    }))
}

/// PUT /api/v1/channels/{channel_id}/e2ee/state — Legacy empty acknowledgement.
///
/// Rejects private state uploads and preserves any historical stored blobs.
#[instrument(skip(state, auth, payload), fields(user_id = %auth.user_id))]
pub async fn update_state(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id_str): Path<String>,
    Json(payload): Json<UpdateStateRequest>,
) -> Result<StatusCode, ApiError> {
    let channel_id = helpers::parse_snowflake(&channel_id_str)?;
    update_state_for_user(&state.db, auth.user_id, channel_id, payload).await
}

async fn update_state_for_user(
    pool: &sqlx::PgPool,
    user_id: Snowflake,
    channel_id: Snowflake,
    payload: UpdateStateRequest,
) -> Result<StatusCode, ApiError> {
    reject_private_state(&payload.group_state)?;
    super::sync::reject_legacy(pool, channel_id).await?;

    permission_check::require_channel_perm(
        pool,
        user_id,
        channel_id,
        Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
    )
    .await?;

    // Keep the legacy route for empty acknowledgements, without rewriting old state.
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM e2ee_groups WHERE channel_id = $1 AND user_id = $2)",
    )
    .bind(channel_id.as_i64())
    .bind(user_id.as_i64())
    .fetch_one(pool)
    .await
    .map_err(ApiError::Database)?;

    if !exists {
        return Err(ApiError::NotFound(
            "no E2EE group found for this channel; call /init first".into(),
        ));
    }

    tracing::debug!(
        user_id = %user_id,
        channel_id = channel_id.as_i64(),
        "local group state acknowledged"
    );
    Ok(StatusCode::NO_CONTENT)
}

fn reject_private_state(value: &str) -> Result<(), ApiError> {
    if !value.is_empty() {
        return Err(ApiError::BadRequest(
            "private MLS group state must remain on the client; omit group_state".into(),
        ));
    }
    Ok(())
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
mod tests;
