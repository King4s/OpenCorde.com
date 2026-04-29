//! # Recording Handlers
//! HTTP handlers for the three recording endpoints.
//!
//! All write operations (start/stop) require server ownership.
//! List is accessible to any authenticated user.
//!
//! ## Depends On
//! - crate::AppState, crate::error::ApiError
//! - crate::middleware::auth::AuthUser
//! - crate::routes::voice::livekit::create_livekit_token
//! - super::types (wire types + DB row)

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::Utc;
use opencorde_core::permissions::Permissions;

use crate::AppState;
use crate::error::ApiError;
use crate::middleware::auth::AuthUser;
use crate::routes::helpers::parse_snowflake;
use crate::routes::permission_check;
use crate::routes::voice::livekit::create_livekit_token;

use super::types::{
    EgressFileOutput, EgressS3Config, EgressStartRequest, EgressStartResponse, EgressStopRequest,
    RecordingResponse, RecordingRow, StartRecordingResponse, row_to_response,
};

const EGRESS_TOKEN_EXPIRY: u64 = 60;

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Resolve server_id and owner_id for a channel.
async fn get_channel_server(state: &AppState, channel_id: i64) -> Result<(i64, i64), ApiError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        server_id: i64,
        owner_id: i64,
    }

    sqlx::query_as::<_, Row>(
        "SELECT c.server_id, s.owner_id \
         FROM channels c JOIN servers s ON s.id = c.server_id \
         WHERE c.id = $1",
    )
    .bind(channel_id)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .map(|r| (r.server_id, r.owner_id))
    .ok_or_else(|| ApiError::NotFound("channel not found".into()))
}

/// Build a short-lived admin JWT for the LiveKit Egress API.
fn egress_token(api_key: &str, api_secret: &str, room_name: &str) -> Result<String, ApiError> {
    create_livekit_token(
        api_key,
        api_secret,
        "egress-bot",
        room_name,
        EGRESS_TOKEN_EXPIRY,
        true,
    )
    .map_err(|e| {
        tracing::error!(error = %e, "egress token generation failed");
        ApiError::Internal(anyhow::anyhow!("token generation failed"))
    })
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

/// POST /api/v1/channels/{id}/recording/start
#[tracing::instrument(skip(state, auth))]
pub async fn start_recording(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id_str): Path<String>,
) -> Result<(StatusCode, Json<StartRecordingResponse>), ApiError> {
    let channel_id = parse_snowflake(&channel_id_str)?;
    let (server_id, _owner_id) = get_channel_server(&state, channel_id.as_i64()).await?;
    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id,
        Permissions::VIEW_CHANNEL | Permissions::MANAGE_CHANNELS,
    )
    .await?;

    let timestamp = Utc::now().timestamp();
    let file_path = format!("recordings/{}/{}.mp4", channel_id.as_i64(), timestamp);
    let room_name = channel_id.as_i64().to_string();
    let token = egress_token(
        &state.config.livekit_api_key,
        &state.config.livekit_api_secret,
        &room_name,
    )?;
    let egress_url = format!(
        "{}/egress/room",
        state.config.livekit_url.trim_end_matches('/')
    );

    let body = EgressStartRequest {
        room_name: room_name.clone(),
        file: EgressFileOutput {
            filepath: file_path.clone(),
            s3: EgressS3Config {
                access_key: state.config.minio_access_key.clone(),
                secret: state.config.minio_secret_key.clone(),
                region: "us-east-1".to_string(),
                endpoint: state.config.minio_endpoint.clone(),
                bucket: state.config.minio_bucket.clone(),
                force_path_style: true,
            },
        },
    };

    tracing::info!(channel_id = channel_id.as_i64(), egress_url = %egress_url, "starting Egress");

    let http = reqwest::Client::new();
    let resp = http
        .post(&egress_url)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "LiveKit Egress unreachable");
            ApiError::Internal(anyhow::anyhow!("LiveKit Egress API unreachable"))
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        tracing::error!(http_status = %status, body = %text, "Egress start failed");
        return Err(ApiError::Internal(anyhow::anyhow!(
            "LiveKit Egress returned {}: {}",
            status,
            text
        )));
    }

    let egress_resp: EgressStartResponse = resp.json().await.map_err(|e| {
        tracing::error!(error = %e, "invalid Egress start response");
        ApiError::Internal(anyhow::anyhow!("invalid Egress response"))
    })?;

    tracing::info!(egress_id = %egress_resp.egress_id, "Egress job started");

    let row = sqlx::query_as::<_, RecordingRow>(
        "INSERT INTO recordings \
         (server_id, channel_id, started_by, egress_id, status, file_path) \
         VALUES ($1, $2, $3, $4, 'recording', $5) \
         RETURNING *",
    )
    .bind(server_id)
    .bind(channel_id.as_i64())
    .bind(auth.user_id.as_i64())
    .bind(&egress_resp.egress_id)
    .bind(&file_path)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok((
        StatusCode::CREATED,
        Json(StartRecordingResponse {
            recording_id: row.id.to_string(),
            egress_id: row.egress_id,
            status: row.status,
        }),
    ))
}

/// POST /api/v1/channels/{id}/recording/stop
#[tracing::instrument(skip(state, auth))]
pub async fn stop_recording(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id_str): Path<String>,
) -> Result<Json<RecordingResponse>, ApiError> {
    let channel_id = parse_snowflake(&channel_id_str)?;
    let (_server_id, _owner_id) = get_channel_server(&state, channel_id.as_i64()).await?;
    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id,
        Permissions::VIEW_CHANNEL | Permissions::MANAGE_CHANNELS,
    )
    .await?;

    let row = sqlx::query_as::<_, RecordingRow>(
        "SELECT * FROM recordings \
         WHERE channel_id = $1 AND status = 'recording' \
         ORDER BY started_at DESC LIMIT 1",
    )
    .bind(channel_id.as_i64())
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("no active recording for this channel".into()))?;

    let room_name = channel_id.as_i64().to_string();
    let token = egress_token(
        &state.config.livekit_api_key,
        &state.config.livekit_api_secret,
        &room_name,
    )?;
    let stop_url = format!(
        "{}/egress/stop",
        state.config.livekit_url.trim_end_matches('/')
    );

    tracing::info!(egress_id = %row.egress_id, stop_url = %stop_url, "stopping Egress");

    let http = reqwest::Client::new();
    let resp = http
        .post(&stop_url)
        .bearer_auth(&token)
        .json(&EgressStopRequest {
            egress_id: row.egress_id.clone(),
        })
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "LiveKit Egress stop unreachable");
            ApiError::Internal(anyhow::anyhow!("LiveKit Egress API unreachable"))
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        tracing::error!(http_status = %status, body = %text, "Egress stop failed");
        return Err(ApiError::Internal(anyhow::anyhow!(
            "LiveKit Egress stop returned {}: {}",
            status,
            text
        )));
    }

    let updated = sqlx::query_as::<_, RecordingRow>(
        "UPDATE recordings \
         SET status = 'stopped', stopped_at = NOW() \
         WHERE id = $1 \
         RETURNING *",
    )
    .bind(row.id)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    tracing::info!(egress_id = %updated.egress_id, "recording stopped");
    Ok(Json(row_to_response(updated)))
}

/// GET /api/v1/channels/{id}/recordings
#[tracing::instrument(skip(state, auth))]
pub async fn list_recordings(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id_str): Path<String>,
) -> Result<Json<Vec<RecordingResponse>>, ApiError> {
    let channel_id = parse_snowflake(&channel_id_str)?;

    get_channel_server(&state, channel_id.as_i64()).await?;
    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id,
        Permissions::VIEW_CHANNEL,
    )
    .await?;

    let rows = sqlx::query_as::<_, RecordingRow>(
        "SELECT * FROM recordings WHERE channel_id = $1 ORDER BY started_at DESC",
    )
    .bind(channel_id.as_i64())
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    tracing::info!(
        channel_id = channel_id.as_i64(),
        count = rows.len(),
        "recordings listed"
    );

    Ok(Json(rows.into_iter().map(row_to_response).collect()))
}
