//! PATCH /api/v1/users/@me/presence — Update user presence (status, custom status, activity).
//! DELETE /api/v1/users/@me/presence/custom — Clear custom status.

use axum::{Json, extract::State};
use chrono::{DateTime, Utc};
use opencorde_db::repos::user_repo;
use serde::{Deserialize, Serialize};

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

/// Request body for updating user presence.
#[derive(Debug, Deserialize)]
pub struct UpdatePresenceRequest {
    /// User status: 0=Online, 1=Idle, 2=DND, 3=Invisible
    pub status: Option<i16>,
    /// Custom status text (emoji + optional message, max 128 chars)
    pub custom_status_text: Option<String>,
    /// When the custom status expires (ISO 8601 timestamptz)
    pub custom_status_expires_at: Option<String>,
    /// Activity type: playing, listening, streaming, custom
    pub activity_type: Option<String>,
    /// Activity name (game, song, etc., max 128 chars)
    pub activity_name: Option<String>,
}

/// Response for presence update.
#[derive(Debug, Serialize)]
pub struct PresenceResponse {
    pub user_id: String,
    pub status: i16,
    pub status_text: String,
    pub custom_status_text: Option<String>,
    pub custom_status_expires_at: Option<String>,
    pub activity_type: Option<String>,
    pub activity_name: Option<String>,
}

/// Build the presence change broadcast event payload.
fn build_presence_event(user_id_str: &str, response: &PresenceResponse) -> serde_json::Value {
    let status_str = &response.status_text;
    let online = response.status != 3; // 3 = Invisible

    let mut data = serde_json::json!({
        "user_id": user_id_str,
        "online": online,
        "status": status_str,
    });

    if let Some(ref text) = response.custom_status_text {
        data["custom_status"] = serde_json::json!({
            "text": text,
            "expires_at": response.custom_status_expires_at,
        });
    }

    if let (Some(atype), Some(aname)) = (&response.activity_type, &response.activity_name) {
        if !atype.is_empty() && !aname.is_empty() {
            data["activity"] = serde_json::json!({
                "type": atype,
                "name": aname,
            });
        }
    }

    serde_json::json!({
        "type": "PresenceUpdate",
        "data": data,
    })
}

fn status_to_text(status: i16) -> &'static str {
    match status {
        0 => "online",
        1 => "idle",
        2 => "dnd",
        3 => "invisible",
        _ => "offline",
    }
}

/// PATCH /api/v1/users/@me/presence — Update user's status, custom status, or activity.
#[tracing::instrument(skip(state, auth, req))]
pub async fn update_presence(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdatePresenceRequest>,
) -> Result<Json<PresenceResponse>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "updating user presence");

    let user_id_str = auth.user_id.to_string();

    // Fetch current user for baseline
    let user_row = user_repo::get_by_id(&state.db, auth.user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            tracing::warn!(user_id = %auth.user_id, "user not found");
            ApiError::NotFound("user not found".into())
        })?;

    // Determine new values (use existing if not provided)
    let new_status = req.status.unwrap_or(user_row.status);
    if !(0..=3).contains(&new_status) {
        return Err(ApiError::BadRequest(
            "status must be 0-3 (0=Online, 1=Idle, 2=DND, 3=Invisible)".into(),
        ));
    }

    let new_custom_status_text = req.custom_status_text.or(user_row.custom_status_text.clone());
    if let Some(ref text) = new_custom_status_text {
        if text.len() > 128 {
            return Err(ApiError::BadRequest(
                "custom_status_text must be 128 characters or less".into(),
            ));
        }
    }

    let new_custom_status_expires_at: Option<DateTime<Utc>> = req
        .custom_status_expires_at
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc)))
        .or(user_row.custom_status_expires_at);

    let new_activity_type = req.activity_type.or(user_row.activity_type.clone());
    if let Some(ref atype) = new_activity_type {
        if !["playing", "listening", "streaming", "custom"].contains(&atype.as_str()) {
            return Err(ApiError::BadRequest(
                "activity_type must be one of: playing, listening, streaming, custom".into(),
            ));
        }
    }

    let new_activity_name = req.activity_name.or(user_row.activity_name.clone());
    if let Some(ref name) = new_activity_name {
        if name.len() > 128 {
            return Err(ApiError::BadRequest(
                "activity_name must be 128 characters or less".into(),
            ));
        }
    }

    // Persist
    user_repo::update_presence(
        &state.db,
        auth.user_id,
        new_status,
        new_custom_status_text.as_deref(),
        new_custom_status_expires_at,
        new_activity_type.as_deref(),
        new_activity_name.as_deref(),
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to update presence");
        ApiError::Database(e)
    })?;

    tracing::info!(user_id = %auth.user_id, status = new_status, "presence updated successfully");

    let response = PresenceResponse {
        user_id: user_id_str.clone(),
        status: new_status,
        status_text: status_to_text(new_status).to_string(),
        custom_status_text: new_custom_status_text.clone(),
        custom_status_expires_at: new_custom_status_expires_at
            .map(|dt| dt.to_rfc3339()),
        activity_type: new_activity_type.clone(),
        activity_name: new_activity_name.clone(),
    };

    // Broadcast presence update via WebSocket
    let presence_event = build_presence_event(&user_id_str, &response);
    state.publish_event(presence_event);

    Ok(Json(response))
}

/// DELETE /api/v1/users/@me/presence/custom — Clear custom status.
#[tracing::instrument(skip(state, auth))]
pub async fn clear_custom_status(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<PresenceResponse>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "clearing custom status");

    let user_row = user_repo::get_by_id(&state.db, auth.user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            tracing::warn!(user_id = %auth.user_id, "user not found");
            ApiError::NotFound("user not found".into())
        })?;

    let user_id_str = auth.user_id.to_string();

    user_repo::update_presence(
        &state.db,
        auth.user_id,
        user_row.status,
        None,                         // clear custom_status_text
        None,                         // clear custom_status_expires_at
        user_row.activity_type.as_deref(),
        user_row.activity_name.as_deref(),
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to clear custom status");
        ApiError::Database(e)
    })?;

    let response = PresenceResponse {
        user_id: user_id_str.clone(),
        status: user_row.status,
        status_text: status_to_text(user_row.status).to_string(),
        custom_status_text: None,
        custom_status_expires_at: None,
        activity_type: user_row.activity_type.clone(),
        activity_name: user_row.activity_name.clone(),
    };

    // Broadcast updated presence
    let presence_event = build_presence_event(&user_id_str, &response);
    state.publish_event(presence_event);

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_to_text_all_values() {
        assert_eq!(status_to_text(0), "online");
        assert_eq!(status_to_text(1), "idle");
        assert_eq!(status_to_text(2), "dnd");
        assert_eq!(status_to_text(3), "invisible");
        assert_eq!(status_to_text(99), "offline"); // invalid
    }

    #[test]
    fn test_build_presence_event_basic() {
        let response = PresenceResponse {
            user_id: "123".to_string(),
            status: 0,
            status_text: "online".to_string(),
            custom_status_text: None,
            custom_status_expires_at: None,
            activity_type: None,
            activity_name: None,
        };
        let event = build_presence_event("123", &response);
        let obj: serde_json::Value = serde_json::from_str(&event.to_string()).unwrap();
        assert_eq!(obj["type"], "PresenceUpdate");
        assert_eq!(obj["data"]["status"], "online");
        assert_eq!(obj["data"]["online"], true);
    }

    #[test]
    fn test_build_presence_event_with_custom_status_and_activity() {
        let response = PresenceResponse {
            user_id: "456".to_string(),
            status: 1,
            status_text: "idle".to_string(),
            custom_status_text: Some("🎮 Gaming".to_string()),
            custom_status_expires_at: None,
            activity_type: Some("playing".to_string()),
            activity_name: Some("Rust".to_string()),
        };
        let event = build_presence_event("456", &response);
        let obj: serde_json::Value = serde_json::from_str(&event.to_string()).unwrap();
        assert_eq!(obj["type"], "PresenceUpdate");
        assert_eq!(obj["data"]["status"], "idle");
        assert_eq!(obj["data"]["online"], true);
        assert_eq!(obj["data"]["custom_status"]["text"], "🎮 Gaming");
        assert_eq!(obj["data"]["activity"]["type"], "playing");
        assert_eq!(obj["data"]["activity"]["name"], "Rust");
    }

    #[test]
    fn test_build_presence_event_invisible_is_offline() {
        let response = PresenceResponse {
            user_id: "789".to_string(),
            status: 3,
            status_text: "invisible".to_string(),
            custom_status_text: None,
            custom_status_expires_at: None,
            activity_type: None,
            activity_name: None,
        };
        let event = build_presence_event("789", &response);
        let obj: serde_json::Value = serde_json::from_str(&event.to_string()).unwrap();
        assert_eq!(obj["data"]["status"], "invisible");
        assert_eq!(obj["data"]["online"], false);
    }

    #[test]
    fn test_update_presence_request_deserialization() {
        let json = r#"{"status": 1, "custom_status_text": "🎮 Gaming", "activity_type": "playing", "activity_name": "Rust"}"#;
        let req: UpdatePresenceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.status, Some(1));
        assert_eq!(req.custom_status_text, Some("🎮 Gaming".to_string()));
        assert_eq!(req.activity_type, Some("playing".to_string()));
        assert_eq!(req.activity_name, Some("Rust".to_string()));
    }

    #[test]
    fn test_update_presence_request_partial() {
        let json = r#"{"status": 2}"#;
        let req: UpdatePresenceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.status, Some(2));
        assert!(req.custom_status_text.is_none());
        assert!(req.activity_type.is_none());
    }
}
