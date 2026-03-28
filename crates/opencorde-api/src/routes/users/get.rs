//! GET /api/v1/users/@me and /api/v1/users/{id} handlers.

use axum::{Json, extract::{State, Path}};
use opencorde_db::repos::user_repo;
use serde::Serialize;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};
use super::super::helpers::parse_snowflake;

/// User profile response.
#[derive(Debug, Serialize)]
pub struct UserProfile {
    pub id: String,
    pub username: String,
    pub public_key: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub status: i16,
    pub bio: Option<String>,
    pub status_message: Option<String>,
    pub totp_enabled: bool,
}

/// Public user profile (no email or public key).
#[derive(Debug, Serialize)]
pub struct PublicUserProfile {
    pub id: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub status: i16,
    pub bio: Option<String>,
    pub status_message: Option<String>,
}

/// GET /api/v1/users/@me — Get authenticated user's profile.
///
/// Requires valid access token in Authorization header.
#[tracing::instrument(skip(state, auth))]
pub async fn get_me(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<UserProfile>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "fetching user profile");

    // Fetch user from database
    let user_row = user_repo::get_by_id(&state.db, auth.user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            tracing::warn!(user_id = %auth.user_id, "user not found");
            ApiError::NotFound("user not found".into())
        })?;

    tracing::debug!(user_id = %auth.user_id, "user profile retrieved");

    let profile = UserProfile {
        id: user_row.id.to_string(),
        username: user_row.username,
        public_key: user_row.public_key,
        email: user_row.email,
        avatar_url: user_row.avatar_url,
        status: user_row.status,
        bio: user_row.bio,
        status_message: user_row.status_message,
        totp_enabled: user_row.totp_enabled,
    };

    Ok(Json(profile))
}

/// GET /api/v1/users/{id} — Get a user's public profile.
///
/// Returns public profile info (no email or public key). Requires authentication.
#[tracing::instrument(skip(state, auth))]
pub async fn get_user_profile(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<PublicUserProfile>, ApiError> {
    tracing::info!(user_id = %auth.user_id, target_id = %id, "fetching public user profile");

    let target_id = parse_snowflake(&id)?;

    let user_row = user_repo::get_by_id(&state.db, target_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            tracing::warn!(target_id = %target_id, "user not found");
            ApiError::NotFound("user not found".into())
        })?;

    tracing::debug!(target_id = %target_id, "public user profile retrieved");

    Ok(Json(PublicUserProfile {
        id: user_row.id.to_string(),
        username: user_row.username,
        avatar_url: user_row.avatar_url,
        status: user_row.status,
        bio: user_row.bio,
        status_message: user_row.status_message,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_user_profile_serialization() {
        let profile = PublicUserProfile {
            id: "456".to_string(),
            username: "publicuser".to_string(),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
            status: 0,
            bio: Some("A public bio".to_string()),
            status_message: None,
        };
        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("publicuser"));
        assert!(json.contains("avatar"));
    }
}
