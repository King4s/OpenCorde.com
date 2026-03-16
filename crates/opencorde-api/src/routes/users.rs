//! # Route: Users
//! User profile retrieval and update endpoints.
//!
//! ## Endpoints
//! - GET /api/v1/users/@me — Get authenticated user's profile
//! - PATCH /api/v1/users/@me — Update authenticated user's profile
//!
//! ## Features
//! - Authentication required (AuthUser extractor)
//! - Conflict detection on email/username updates
//! - Input validation (username, email format)
//! - Comprehensive structured logging
//!
//! ## Depends On
//! - axum (web framework)
//! - crate::middleware::auth::AuthUser (authentication extractor)
//! - opencorde_db::repos::user_repo (CRUD operations)
//! - crate::AppState (database + config)
//! - crate::error::ApiError (unified error handling)

use axum::{Json, Router, extract::State, routing::get};
use opencorde_db::repos::user_repo;
use serde::{Deserialize, Serialize};

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

/// User profile response.
#[derive(Debug, Serialize)]
pub struct UserProfile {
    /// Snowflake user ID
    pub id: String,
    /// Username
    pub username: String,
    /// Ed25519 public key (global mesh identity)
    pub public_key: String,
    /// Email address (optional)
    pub email: Option<String>,
    /// Avatar URL (if set)
    pub avatar_url: Option<String>,
    /// User status (0=Online, 1=Idle, 2=DND, 3=Offline)
    pub status: i16,
}

/// Request body for updating user profile.
/// All fields are optional.
#[derive(Debug, Deserialize)]
pub struct UpdateMeRequest {
    /// New username
    pub username: Option<String>,
    /// New email address
    pub email: Option<String>,
}

/// Build the users router.
pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/users/@me", get(get_me).patch(update_me))
}

/// Validate username format.
fn validate_username(username: &str) -> Result<(), ApiError> {
    if username.len() < 3 || username.len() > 32 {
        return Err(ApiError::BadRequest(
            "username must be 3-32 characters".into(),
        ));
    }

    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(ApiError::BadRequest(
            "username must be alphanumeric or underscore".into(),
        ));
    }

    Ok(())
}

/// Validate email format.
fn validate_email(email: &str) -> Result<(), ApiError> {
    if !email.contains('@') || email.len() < 5 {
        return Err(ApiError::BadRequest("invalid email address".into()));
    }

    Ok(())
}

/// GET /api/v1/users/@me — Get authenticated user's profile.
///
/// Requires valid access token in Authorization header.
#[tracing::instrument(skip(state, auth))]
async fn get_me(
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
    };

    Ok(Json(profile))
}

/// PATCH /api/v1/users/@me — Update authenticated user's profile.
///
/// Allows updating username and email. Validates for conflicts and format.
#[tracing::instrument(skip(state, auth, req))]
async fn update_me(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdateMeRequest>,
) -> Result<Json<UserProfile>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "updating user profile");

    // Fetch current user from database
    let mut user_row = user_repo::get_by_id(&state.db, auth.user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            tracing::warn!(user_id = %auth.user_id, "user not found");
            ApiError::NotFound("user not found".into())
        })?;

    // Validate and check new username if provided
    if let Some(ref new_username) = req.username {
        validate_username(new_username)?;

        // Check for conflict if username changed
        if new_username != &user_row.username {
            let existing = user_repo::get_by_username(&state.db, new_username)
                .await
                .map_err(ApiError::Database)?;

            if existing.is_some() {
                tracing::warn!(username = %new_username, "username already taken");
                return Err(ApiError::Conflict("username already taken".into()));
            }

            user_row.username = new_username.clone();
            tracing::debug!(user_id = %auth.user_id, username = %new_username, "username updated");
        }
    }

    // Validate and check new email if provided
    if let Some(ref new_email) = req.email {
        validate_email(new_email)?;

        // Check for conflict if email changed
        if Some(new_email) != user_row.email.as_ref() {
            let existing = user_repo::get_by_email(&state.db, new_email)
                .await
                .map_err(ApiError::Database)?;

            if existing.is_some() {
                tracing::warn!(email = %new_email, "email already registered");
                return Err(ApiError::Conflict("email already registered".into()));
            }

            user_row.email = Some(new_email.clone());
            tracing::debug!(user_id = %auth.user_id, email = %new_email, "email updated");
        }
    }

    // If no changes needed, skip database update
    let has_changes = req.username.is_some() || req.email.is_some();

    if has_changes {
        // Build dynamic UPDATE query (simple approach: update both columns always)
        sqlx::query("UPDATE users SET username = $1, email = $2, updated_at = NOW() WHERE id = $3")
            .bind(&user_row.username)
            .bind(&user_row.email)
            .bind(auth.user_id.as_i64())
            .execute(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "failed to update user");
                ApiError::Database(e)
            })?;

        tracing::info!(user_id = %auth.user_id, "user profile updated successfully");
    } else {
        tracing::debug!(user_id = %auth.user_id, "no changes to apply");
    }

    let profile = UserProfile {
        id: user_row.id.to_string(),
        username: user_row.username,
        public_key: user_row.public_key,
        email: user_row.email,
        avatar_url: user_row.avatar_url,
        status: user_row.status,
    };

    Ok(Json(profile))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_username() {
        assert!(validate_username("validuser").is_ok());
        assert!(validate_username("valid_user_123").is_ok());
        assert!(validate_username("ab").is_err()); // too short
        assert!(validate_username(&"a".repeat(33)).is_err()); // too long
        assert!(validate_username("invalid-user").is_err());
        assert!(validate_username("invalid user").is_err());
    }

    #[test]
    fn test_validate_email() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("a@b.c").is_ok());
        assert!(validate_email("notanemail").is_err());
        assert!(validate_email("a@bc").is_err());
    }

    #[test]
    fn test_user_profile_serialization() {
        let profile = UserProfile {
            id: "123".to_string(),
            username: "test".to_string(),
            public_key: "abc123def456abc123def456abc123def456abc123def456abc123def456abc1"
                .to_string(),
            email: Some("t@e.com".to_string()),
            avatar_url: Some("https://x.com/a.png".to_string()),
            status: 0,
        };
        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("123"));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_update_request() {
        let partial = UpdateMeRequest {
            username: Some("new".into()),
            email: None,
        };
        assert!(partial.username.is_some());
        assert!(partial.email.is_none());

        let empty = UpdateMeRequest {
            username: None,
            email: None,
        };
        assert!(empty.username.is_none());
    }
}
