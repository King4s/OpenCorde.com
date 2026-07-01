//! GET/POST/DELETE /api/v1/users/@me/sessions — manage active sessions.
//!
//! Sessions are derived from the refresh_tokens table (JTI-based).
//! Each non-revoked, non-expired refresh token represents an active session.
//! The currently-authenticated session is identified by its JTI.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::error::ApiError;
use crate::middleware::auth::AuthUser;
use crate::AppState;
use opencorde_db::repos::refresh_token_repo;

#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub current: bool,
}

/// GET /api/v1/users/@me/sessions — list all active sessions for the authenticated user.
pub async fn list_sessions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<SessionInfo>>, ApiError> {
    let rows = refresh_token_repo::list_active_for_user(&state.db, auth.user_id.as_i64())
        .await
        .map_err(ApiError::Database)?;

    let sessions: Vec<SessionInfo> = rows
        .into_iter()
        .map(|r| {
            let current = auth.session_jti.as_deref() == Some(r.jti.as_str());
            SessionInfo {
                id: r.jti.clone(),
                created_at: r.created_at,
                expires_at: r.expires_at,
                current,
            }
        })
        .collect();

    Ok(Json(sessions))
}

/// DELETE /api/v1/users/@me/sessions/{jti} — revoke a specific session.
pub async fn revoke_session(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(jti): Path<String>,
) -> Result<StatusCode, ApiError> {
    // Verify the JTI belongs to the authenticated user
    let row = refresh_token_repo::get_by_jti(&state.db, &jti)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("session not found".into()))?;

    if row.user_id != auth.user_id.as_i64() {
        return Err(ApiError::Forbidden);
    }

    refresh_token_repo::revoke(&state.db, &jti)
        .await
        .map_err(ApiError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/users/@me/sessions — revoke all other sessions (keep current).
pub async fn revoke_other_sessions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<StatusCode, ApiError> {
    let keep_jti = auth.session_jti.as_deref().ok_or_else(|| {
        tracing::warn!(user_id = %auth.user_id, "revoke-others requested without a session jti");
        ApiError::BadRequest("current session cannot be identified".into())
    })?;

    refresh_token_repo::revoke_all_for_user_except(&state.db, auth.user_id.as_i64(), keep_jti)
        .await
        .map_err(ApiError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}
