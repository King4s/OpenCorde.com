//! GET  /api/v1/users/@me/activities       — list my activities
//! POST /api/v1/users/@me/activities       — create a new activity
//! DELETE /api/v1/users/@me/activities/{id} — delete a specific activity

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use opencorde_core::models::ActivityData;
use opencorde_core::snowflake::Snowflake;
use opencorde_db::repos::user_activity_repo;
use serde::{Deserialize, Serialize};

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

/// Request body for creating an activity.
#[derive(Debug, Deserialize)]
pub struct CreateActivityRequest {
    /// Activity type: playing, listening, streaming, custom
    #[serde(rename = "type")]
    pub activity_type: String,
    /// Activity name (game name, song title, etc.)
    pub name: String,
    /// Optional details (e.g. "Competitive - 3v3", "by Artist X")
    pub details: Option<String>,
    /// Optional state (e.g. "In Menu", "Paused", "Live")
    pub state: Option<String>,
    /// Optional end time (ISO 8601)
    pub ends_at: Option<String>,
    /// Optional asset image URL
    pub asset_url: Option<String>,
}

/// GET /api/v1/users/@me/activities — List current user's activities (newest first).
#[tracing::instrument(skip(state, auth))]
pub async fn list_my_activities(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<ActivityData>>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "listing user activities");

    let rows = user_activity_repo::get_by_user(&state.db, auth.user_id, 20)
        .await
        .map_err(ApiError::Database)?;

    let activities: Vec<ActivityData> = rows
        .into_iter()
        .map(|row| ActivityData {
            id: Snowflake::new(row.id),
            user_id: Snowflake::new(row.user_id),
            activity_type: row.activity_type,
            name: row.name,
            details: row.details,
            state: row.state,
            started_at: row.started_at,
            ends_at: row.ends_at,
            asset_url: row.asset_url,
        })
        .collect();

    Ok(Json(activities))
}

/// POST /api/v1/users/@me/activities — Create a new activity.
///
/// Validates type against the allowed enum: playing, listening, streaming, custom.
#[tracing::instrument(skip(state, auth, req))]
pub async fn create_my_activity(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateActivityRequest>,
) -> Result<(StatusCode, Json<ActivityData>), ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        activity_type = %req.activity_type,
        name = %req.name,
        "creating user activity"
    );

    // Validate activity type
    if !["playing", "listening", "streaming", "custom"].contains(&req.activity_type.as_str()) {
        return Err(ApiError::BadRequest(
            "activity_type must be one of: playing, listening, streaming, custom".into(),
        ));
    }

    if req.name.trim().is_empty() {
        return Err(ApiError::BadRequest("name is required".into()));
    }

    if req.name.len() > 128 {
        return Err(ApiError::BadRequest(
            "name must be 128 characters or less".into(),
        ));
    }

    let activity_id = Snowflake::generate();

    let ends_at: Option<DateTime<Utc>> = req
        .ends_at
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc)));

    let row = user_activity_repo::create_activity(
        &state.db,
        activity_id,
        auth.user_id,
        &req.activity_type,
        req.name.trim(),
        req.details.as_deref(),
        req.state.as_deref(),
        ends_at,
        req.asset_url.as_deref(),
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to create activity");
        ApiError::Database(e)
    })?;

    let activity_data = ActivityData {
        id: Snowflake::new(row.id),
        user_id: Snowflake::new(row.user_id),
        activity_type: row.activity_type,
        name: row.name,
        details: row.details,
        state: row.state,
        started_at: row.started_at,
        ends_at: row.ends_at,
        asset_url: row.asset_url,
    };

    // Broadcast ActivityUpdate over WebSocket
    let event = serde_json::json!({
        "type": "ActivityUpdate",
        "data": activity_data,
    });
    state.publish_event(event);

    tracing::info!(
        activity_id = row.id,
        "activity created and broadcast"
    );

    Ok((StatusCode::CREATED, Json(activity_data)))
}

/// DELETE /api/v1/users/@me/activities/{id} — Delete a specific activity.
#[tracing::instrument(skip(state, auth))]
pub async fn delete_my_activity(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(user_id = %auth.user_id, activity_id = %id, "deleting user activity");

    let activity_id = id
        .parse::<i64>()
        .map(Snowflake::new)
        .map_err(|_| ApiError::BadRequest("invalid activity id".into()))?;

    let deleted = user_activity_repo::delete_activity(&state.db, activity_id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to delete activity");
            ApiError::Database(e)
        })?;

    if !deleted {
        return Err(ApiError::NotFound("activity not found".into()));
    }

    tracing::info!(activity_id = %id, "activity deleted");

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_activity_request_deserialization() {
        let json = r#"{"type":"playing","name":"Rust","details":"Competitive - 3v3","state":"In Menu"}"#;
        let req: CreateActivityRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.activity_type, "playing");
        assert_eq!(req.name, "Rust");
        assert_eq!(req.details, Some("Competitive - 3v3".to_string()));
        assert_eq!(req.state, Some("In Menu".to_string()));
    }

    #[test]
    fn test_create_activity_request_minimal() {
        let json = r#"{"type":"custom","name":"Coding"}"#;
        let req: CreateActivityRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.activity_type, "custom");
        assert_eq!(req.name, "Coding");
        assert!(req.details.is_none());
        assert!(req.state.is_none());
    }

    #[test]
    fn test_activity_data_has_required_fields() {
        use chrono::Utc;
        let data = ActivityData {
            id: Snowflake::new(1),
            user_id: Snowflake::new(42),
            activity_type: "playing".to_string(),
            name: "Rust".to_string(),
            details: None,
            state: None,
            started_at: Utc::now(),
            ends_at: None,
            asset_url: None,
        };
        assert_eq!(data.activity_type, "playing");
        assert_eq!(data.name, "Rust");
    }
}
