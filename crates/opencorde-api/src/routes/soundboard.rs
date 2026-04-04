//! # Route: Soundboard
//! Manage per-server soundboard sounds.
//!
//! ## Endpoints
//! - GET  /api/v1/servers/{server_id}/soundboard        — list sounds
//! - POST /api/v1/servers/{server_id}/soundboard        — upload sound (owner only)
//! - DELETE /api/v1/servers/{server_id}/soundboard/{id} — delete sound (owner only)
//! - POST /api/v1/servers/{server_id}/soundboard/{id}/play — play sound (broadcast WS event)
//!
//! ## Depends On
//! - sqlx, AppState, ApiError, AuthUser

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};
use crate::routes::helpers::{check_server_owner, parse_snowflake};
use opencorde_db::repos::server_repo;

#[derive(Debug, Serialize)]
pub struct SoundResponse {
    pub id: String,
    pub server_id: String,
    pub name: String,
    pub file_key: String,
    pub uploader_id: String,
    pub volume: i16,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSoundRequest {
    pub name: String,
    pub file_key: String,
    pub volume: Option<i16>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/servers/{server_id}/soundboard",
            get(list_sounds).post(create_sound),
        )
        .route(
            "/api/v1/servers/{server_id}/soundboard/{sound_id}",
            delete(delete_sound),
        )
        .route(
            "/api/v1/servers/{server_id}/soundboard/{sound_id}/play",
            post(play_sound),
        )
}

#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn list_sounds(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
) -> Result<Json<Vec<SoundResponse>>, ApiError> {
    let sid = parse_snowflake(&server_id)?;
    // Must be a member
    let rows = sqlx::query(
        "SELECT id, server_id, name, file_key, uploader_id, volume, created_at \
         FROM soundboard_sounds WHERE server_id = $1 ORDER BY name ASC",
    )
    .bind(sid.as_i64())
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let sounds = rows.iter().map(|r| SoundResponse {
        id: r.get::<i64, _>("id").to_string(),
        server_id: r.get::<i64, _>("server_id").to_string(),
        name: r.get("name"),
        file_key: r.get("file_key"),
        uploader_id: r.get::<i64, _>("uploader_id").to_string(),
        volume: r.get("volume"),
        created_at: r.get("created_at"),
    }).collect();

    Ok(Json(sounds))
}

#[tracing::instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
async fn create_sound(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
    Json(req): Json<CreateSoundRequest>,
) -> Result<(StatusCode, Json<SoundResponse>), ApiError> {
    let sid = parse_snowflake(&server_id)?;
    let server = server_repo::get_by_id(&state.db, sid)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;
    check_server_owner(auth.user_id, server.owner_id)?;

    if req.name.is_empty() || req.name.len() > 32 {
        return Err(ApiError::BadRequest("name must be 1-32 characters".into()));
    }

    let id = opencorde_core::snowflake::SnowflakeGenerator::new(5, 0).next_id();
    let volume = req.volume.unwrap_or(100).clamp(10, 100);

    let row = sqlx::query(
        "INSERT INTO soundboard_sounds (id, server_id, name, file_key, uploader_id, volume) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, server_id, name, file_key, uploader_id, volume, created_at",
    )
    .bind(id.as_i64())
    .bind(sid.as_i64())
    .bind(&req.name)
    .bind(&req.file_key)
    .bind(auth.user_id.as_i64())
    .bind(volume)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("unique") {
            ApiError::Conflict("a sound with that name already exists".into())
        } else {
            ApiError::Internal(e.into())
        }
    })?;

    tracing::info!(server_id = sid.as_i64(), name = %req.name, "soundboard sound created");

    Ok((StatusCode::CREATED, Json(SoundResponse {
        id: row.get::<i64, _>("id").to_string(),
        server_id: row.get::<i64, _>("server_id").to_string(),
        name: row.get("name"),
        file_key: row.get("file_key"),
        uploader_id: row.get::<i64, _>("uploader_id").to_string(),
        volume: row.get("volume"),
        created_at: row.get("created_at"),
    })))
}

#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn delete_sound(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, sound_id)): Path<(String, i64)>,
) -> Result<StatusCode, ApiError> {
    let sid = parse_snowflake(&server_id)?;
    let server = server_repo::get_by_id(&state.db, sid)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;
    check_server_owner(auth.user_id, server.owner_id)?;

    let result = sqlx::query(
        "DELETE FROM soundboard_sounds WHERE id = $1 AND server_id = $2",
    )
    .bind(sound_id)
    .bind(sid.as_i64())
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("sound not found".into()));
    }

    tracing::info!(sound_id, server_id = sid.as_i64(), "soundboard sound deleted");
    Ok(StatusCode::NO_CONTENT)
}

#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn play_sound(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, sound_id)): Path<(String, i64)>,
) -> Result<StatusCode, ApiError> {
    let sid = parse_snowflake(&server_id)?;

    // Fetch sound to verify it exists
    let row = sqlx::query(
        "SELECT name, file_key, volume FROM soundboard_sounds WHERE id = $1 AND server_id = $2",
    )
    .bind(sound_id)
    .bind(sid.as_i64())
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?
    .ok_or_else(|| ApiError::NotFound("sound not found".into()))?;

    let name: String = row.get("name");
    let file_key: String = row.get("file_key");
    let volume: i16 = row.get("volume");

    // Broadcast SoundboardPlay WebSocket event to all server members
    let event = serde_json::json!({
        "type": "SoundboardPlay",
        "server_id": sid.as_i64().to_string(),
        "sound_id": sound_id.to_string(),
        "name": name,
        "file_key": file_key,
        "volume": volume,
        "user_id": auth.user_id.as_i64().to_string(),
    });

    // Send to all subscribers of this server's event channel
    let _ = state.event_tx.send(event);

    tracing::info!(sound_id, server_id = sid.as_i64(), "soundboard sound played");
    Ok(StatusCode::NO_CONTENT)
}
