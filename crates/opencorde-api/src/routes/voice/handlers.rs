//! # Voice Route Handlers
//! HTTP handlers for voice channel management endpoints.
//!
//! Implements all voice route handlers with authentication,
//! validation, and LiveKit token generation.

use axum::Json;
use axum::extract::{Path, State};
use opencorde_core::permissions::Permissions;
use opencorde_db::repos::voice_state_repo;
use uuid::Uuid;

use crate::AppState;
use crate::error::ApiError;
use crate::middleware::auth::AuthUser;
use crate::routes::{helpers::parse_snowflake, permission_check};

use super::livekit;
use super::types::*;

const LIVEKIT_TOKEN_EXPIRY: u64 = 3600; // 1 hour

/// POST /api/v1/voice/join — Join a voice channel and receive LiveKit token.
///
/// Requires authentication. Validates channel exists and is voice type (channel_type=1).
/// Creates voice state and generates LiveKit access token.
///
/// # Request
/// ```json
/// {"channel_id": "123456789"}
/// ```
///
/// # Response (200)
/// ```json
/// {
///   "voice_state": {...},
///   "livekit_token": "eyJhbGc..."
/// }
/// ```
#[tracing::instrument(skip(state, auth, req))]
pub async fn join_voice(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<JoinVoiceRequest>,
) -> Result<Json<JoinVoiceResponse>, ApiError> {
    tracing::info!(user_id = %auth.user_id, channel_id = %req.channel_id, "user joining voice channel");

    // Parse channel ID
    let channel_id = parse_snowflake(&req.channel_id)?;

    // Validate channel exists and is voice type
    #[derive(sqlx::FromRow)]
    struct Channel {
        channel_type: i16,
    }

    let channel = sqlx::query_as::<_, Channel>("SELECT channel_type FROM channels WHERE id = $1")
        .bind(channel_id.as_i64())
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            tracing::warn!(channel_id = %channel_id, "channel not found");
            ApiError::NotFound("channel not found".into())
        })?;

    if channel.channel_type != 1 {
        tracing::warn!(channel_id = %channel_id, channel_type = channel.channel_type, "invalid channel type for voice");
        return Err(ApiError::BadRequest(
            "channel is not a voice channel".into(),
        ));
    }

    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id,
        Permissions::CONNECT,
    )
    .await?;

    // Create session ID (UUID)
    let session_id = Uuid::new_v4().to_string();

    // Insert/update voice state
    let voice_state =
        voice_state_repo::join_voice(&state.db, auth.user_id, channel_id, &session_id)
            .await
            .map_err(ApiError::Database)?;

    // Generate LiveKit token
    let livekit_token = livekit::create_livekit_token(
        &state.config.livekit_api_key,
        &state.config.livekit_api_secret,
        &auth.user_id.as_i64().to_string(),
        &channel_id.as_i64().to_string(),
        LIVEKIT_TOKEN_EXPIRY,
    )
    .map_err(|e| {
        tracing::error!(error = %e, "failed to generate LiveKit token");
        ApiError::Internal(anyhow::anyhow!("token generation failed"))
    })?;

    tracing::info!(user_id = %auth.user_id, channel_id = %channel_id, "voice join successful");

    Ok(Json(JoinVoiceResponse {
        voice_state: VoiceStateResponse {
            user_id: auth.user_id.as_i64().to_string(),
            channel_id: voice_state.channel_id.to_string(),
            session_id: voice_state.session_id,
            self_mute: voice_state.self_mute,
            self_deaf: voice_state.self_deaf,
            joined_at: voice_state.joined_at,
        },
        livekit_token,
        livekit_url: state.config.livekit_public_url.clone(),
    }))
}

/// POST /api/v1/voice/leave — Leave the current voice channel.
///
/// Requires authentication. Deletes user's voice state.
/// Returns 204 No Content on success.
#[tracing::instrument(skip(state, auth))]
pub async fn leave_voice(State(state): State<AppState>, auth: AuthUser) -> Result<(), ApiError> {
    tracing::info!(user_id = %auth.user_id, "user leaving voice channel");

    voice_state_repo::leave_voice(&state.db, auth.user_id)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(user_id = %auth.user_id, "voice leave successful");
    Ok(())
}

/// PATCH /api/v1/voice/state — Update voice state (mute/deafen).
///
/// Requires authentication and active voice channel connection.
///
/// # Request
/// ```json
/// {"self_mute": true, "self_deaf": false}
/// ```
#[tracing::instrument(skip(state, auth, req))]
pub async fn update_voice_state(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdateVoiceStateRequest>,
) -> Result<Json<VoiceStateResponse>, ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        self_mute = req.self_mute,
        self_deaf = req.self_deaf,
        "updating voice state"
    );

    // User must be in a voice channel
    let _voice_state = voice_state_repo::get_by_user(&state.db, auth.user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            tracing::warn!(user_id = %auth.user_id, "user not in voice channel");
            ApiError::BadRequest("user is not in a voice channel".into())
        })?;

    // Update state
    let updated =
        voice_state_repo::update_state(&state.db, auth.user_id, req.self_mute, req.self_deaf)
            .await
            .map_err(ApiError::Database)?;

    tracing::info!(user_id = %auth.user_id, "voice state updated");

    Ok(Json(VoiceStateResponse {
        user_id: auth.user_id.as_i64().to_string(),
        channel_id: updated.channel_id.to_string(),
        session_id: updated.session_id,
        self_mute: updated.self_mute,
        self_deaf: updated.self_deaf,
        joined_at: updated.joined_at,
    }))
}

/// GET /api/v1/voice/participants/{channel_id} — List voice channel participants.
///
/// Requires authentication.
#[tracing::instrument(skip(state, auth))]
pub async fn get_participants(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
) -> Result<Json<Vec<VoiceParticipant>>, ApiError> {
    tracing::info!(user_id = %auth.user_id, channel_id = %channel_id, "fetching voice participants");

    let channel_id = parse_snowflake(&channel_id)?;

    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id,
        Permissions::CONNECT,
    )
    .await?;

    // Validate channel exists
    sqlx::query("SELECT id FROM channels WHERE id = $1")
        .bind(channel_id.as_i64())
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            tracing::warn!(channel_id = %channel_id, "channel not found");
            ApiError::NotFound("channel not found".into())
        })?;

    // Get participants
    let participants = voice_state_repo::get_channel_participants(&state.db, channel_id)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(
        channel_id = %channel_id,
        participant_count = participants.len(),
        "participants fetched"
    );

    let response = participants
        .into_iter()
        .map(|p| VoiceParticipant {
            user_id: p.user_id.to_string(),
            session_id: p.session_id,
            self_mute: p.self_mute,
            self_deaf: p.self_deaf,
            joined_at: p.joined_at,
        })
        .collect();

    Ok(Json(response))
}

/// POST /api/v1/livekit/token — Get a fresh LiveKit access token.
///
/// Requires authentication and active voice channel connection in the specified channel.
///
/// # Request
/// ```json
/// {"channel_id": "123456789"}
/// ```
///
/// # Response (200)
/// ```json
/// {
///   "token": "eyJhbGc...",
///   "url": "wss://livekit.example.com"
/// }
/// ```
#[tracing::instrument(skip(state, auth, req))]
pub async fn get_livekit_token(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<LiveKitTokenRequest>,
) -> Result<Json<LiveKitTokenResponse>, ApiError> {
    tracing::info!(user_id = %auth.user_id, channel_id = %req.channel_id, "requesting LiveKit token");

    let channel_id = parse_snowflake(&req.channel_id)?;

    // User must be in voice and in the specified channel
    let voice_state = voice_state_repo::get_by_user(&state.db, auth.user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            tracing::warn!(user_id = %auth.user_id, "user not in voice channel");
            ApiError::BadRequest("user is not in a voice channel".into())
        })?;

    if voice_state.channel_id != channel_id.as_i64() {
        tracing::warn!(
            user_id = %auth.user_id,
            current_channel = voice_state.channel_id,
            requested_channel = channel_id.as_i64(),
            "user not in requested channel"
        );
        return Err(ApiError::Forbidden);
    }

    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id,
        Permissions::CONNECT,
    )
    .await?;

    // Generate token
    let token = livekit::create_livekit_token(
        &state.config.livekit_api_key,
        &state.config.livekit_api_secret,
        &auth.user_id.as_i64().to_string(),
        &channel_id.as_i64().to_string(),
        LIVEKIT_TOKEN_EXPIRY,
    )
    .map_err(|e| {
        tracing::error!(error = %e, "failed to generate LiveKit token");
        ApiError::Internal(anyhow::anyhow!("token generation failed"))
    })?;

    tracing::info!(user_id = %auth.user_id, channel_id = %channel_id, "LiveKit token generated");

    Ok(Json(LiveKitTokenResponse {
        token,
        url: state.config.livekit_url.clone(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_livekit_token_expiry_constant() {
        assert_eq!(LIVEKIT_TOKEN_EXPIRY, 3600);
    }
}
