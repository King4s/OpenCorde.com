//! # Stage Handlers
//! HTTP request handlers for stage channel endpoints.
//!
//! ## Depends On
//! - axum, opencorde_db::repos::stage_repo, crate::AppState, super::types

use axum::{extract::{Path, State}, Json};
use opencorde_db::repos::stage_repo;
use crate::{error::ApiError, middleware::auth::AuthUser, routes::helpers::parse_snowflake, AppState};
use super::types::*;

/// POST /api/v1/channels/{channel_id}/stage/start — Start a stage session (server owner only).
#[tracing::instrument(skip(state, auth))]
pub async fn start_stage(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
    Json(req): Json<StartStageRequest>,
) -> Result<Json<StageSessionResponse>, ApiError> {
    tracing::info!(channel_id = %channel_id, "starting stage session");
    let channel_id_sf = parse_snowflake(&channel_id)?;

    #[derive(sqlx::FromRow)]
    struct ChannelCheck { server_id: i64, channel_type: i16 }

    let ch = sqlx::query_as::<_, ChannelCheck>(
        "SELECT server_id, channel_type FROM channels WHERE id = $1"
    )
    .bind(channel_id_sf.as_i64())
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("channel not found".into()))?;

    if ch.channel_type != 3 {
        return Err(ApiError::BadRequest("channel is not a stage channel".into()));
    }

    let owner_id = sqlx::query_scalar::<_, i64>("SELECT owner_id FROM servers WHERE id = $1")
        .bind(ch.server_id)
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;

    if auth.user_id.as_i64() != owner_id {
        return Err(ApiError::Forbidden);
    }

    let mut sg = opencorde_core::snowflake::SnowflakeGenerator::new(1, 1);
    let session_id = sg.next_id();
    let session = stage_repo::start_session(&state.db, session_id, channel_id_sf, req.topic.as_deref(), auth.user_id)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(session_id = session.id, "stage session started");
    Ok(Json(StageSessionResponse {
        id: session.id.to_string(),
        channel_id: session.channel_id.to_string(),
        topic: session.topic,
        started_by: session.started_by.to_string(),
        started_at: session.started_at,
    }))
}

/// DELETE /api/v1/channels/{channel_id}/stage — End the stage session (starter only).
#[tracing::instrument(skip(state, auth))]
pub async fn end_stage(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
) -> Result<(), ApiError> {
    tracing::info!(channel_id = %channel_id, "ending stage session");
    let channel_id_sf = parse_snowflake(&channel_id)?;

    let session = stage_repo::get_session(&state.db, channel_id_sf)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("no active stage session".into()))?;

    if session.started_by != auth.user_id.as_i64() {
        return Err(ApiError::Forbidden);
    }

    stage_repo::end_session(&state.db, channel_id_sf)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(channel_id = %channel_id, "stage session ended");
    Ok(())
}

/// GET /api/v1/channels/{channel_id}/stage — Get session + participants.
#[tracing::instrument(skip(state, auth))]
pub async fn get_stage(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
) -> Result<Json<StageDetailResponse>, ApiError> {
    let _ = auth;
    let channel_id_sf = parse_snowflake(&channel_id)?;

    let session = stage_repo::get_session(&state.db, channel_id_sf)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("no active stage session".into()))?;

    let participants = stage_repo::list_participants(&state.db, channel_id_sf)
        .await
        .map_err(ApiError::Database)?;

    Ok(Json(StageDetailResponse {
        session: StageSessionResponse {
            id: session.id.to_string(),
            channel_id: session.channel_id.to_string(),
            topic: session.topic,
            started_by: session.started_by.to_string(),
            started_at: session.started_at,
        },
        participants: participants.into_iter().map(|p| StageParticipantResponse {
            id: p.id.to_string(),
            user_id: p.user_id.to_string(),
            username: p.username,
            role: p.role,
            hand_raised: p.hand_raised,
            joined_at: p.joined_at,
        }).collect(),
    }))
}

/// POST /api/v1/channels/{channel_id}/stage/join — Join as audience.
#[tracing::instrument(skip(state, auth))]
pub async fn join_stage(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
) -> Result<Json<StageParticipantResponse>, ApiError> {
    tracing::info!(channel_id = %channel_id, "user joining stage");
    let channel_id_sf = parse_snowflake(&channel_id)?;

    stage_repo::get_session(&state.db, channel_id_sf)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("no active stage session".into()))?;

    let mut sg = opencorde_core::snowflake::SnowflakeGenerator::new(1, 1);
    let p = stage_repo::join_stage(&state.db, sg.next_id(), channel_id_sf, auth.user_id)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(channel_id = %channel_id, "user joined stage");
    Ok(Json(StageParticipantResponse {
        id: p.id.to_string(), user_id: p.user_id.to_string(), username: p.username,
        role: p.role, hand_raised: p.hand_raised, joined_at: p.joined_at,
    }))
}

/// DELETE /api/v1/channels/{channel_id}/stage/leave — Leave the stage.
#[tracing::instrument(skip(state, auth))]
pub async fn leave_stage(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
) -> Result<(), ApiError> {
    let channel_id_sf = parse_snowflake(&channel_id)?;
    stage_repo::leave_stage(&state.db, channel_id_sf, auth.user_id)
        .await
        .map_err(ApiError::Database)?;
    tracing::info!(channel_id = %channel_id, "user left stage");
    Ok(())
}

/// POST /api/v1/channels/{channel_id}/stage/hand — Raise/lower hand.
#[tracing::instrument(skip(state, auth))]
pub async fn toggle_hand(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
    Json(req): Json<HandRequest>,
) -> Result<(), ApiError> {
    let channel_id_sf = parse_snowflake(&channel_id)?;
    if req.raised {
        stage_repo::raise_hand(&state.db, channel_id_sf, auth.user_id).await
    } else {
        stage_repo::lower_hand(&state.db, channel_id_sf, auth.user_id).await
    }
    .map_err(ApiError::Database)?;
    tracing::info!(channel_id = %channel_id, raised = req.raised, "hand toggled");
    Ok(())
}

/// PATCH /api/v1/channels/{channel_id}/stage/speakers/{user_id} — Promote/demote (starter only).
#[tracing::instrument(skip(state, auth))]
pub async fn set_speaker(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((channel_id, target_user_id)): Path<(String, String)>,
    Json(req): Json<SpeakerRequest>,
) -> Result<(), ApiError> {
    tracing::info!(channel_id = %channel_id, target = %target_user_id, "changing speaker role");
    let channel_id_sf = parse_snowflake(&channel_id)?;
    let target_sf = parse_snowflake(&target_user_id)?;

    let session = stage_repo::get_session(&state.db, channel_id_sf)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("no active stage session".into()))?;

    if session.started_by != auth.user_id.as_i64() {
        return Err(ApiError::Forbidden);
    }

    if req.speaker {
        stage_repo::promote_to_speaker(&state.db, channel_id_sf, target_sf).await
    } else {
        stage_repo::demote_to_audience(&state.db, channel_id_sf, target_sf).await
    }
    .map_err(ApiError::Database)?;

    tracing::info!(channel_id = %channel_id, speaker = req.speaker, "speaker role changed");
    Ok(())
}
