//! PATCH /api/v1/users/@me handler for updating user profile.

use axum::{Json, extract::State};
use opencorde_db::repos::user_repo;
use serde::Deserialize;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};
use super::get::UserProfile;

/// Request body for updating user profile.
#[derive(Debug, Deserialize)]
pub struct UpdateMeRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub bio: Option<String>,
    pub status_message: Option<String>,
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

/// PATCH /api/v1/users/@me — Update authenticated user's profile.
///
/// Allows updating username and email. Validates for conflicts and format.
#[tracing::instrument(skip(state, auth, req))]
pub async fn update_me(
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

    // Handle bio update
    if let Some(ref bio) = req.bio {
        if bio.len() > 500 {
            return Err(ApiError::BadRequest("bio must be 500 characters or less".into()));
        }
        user_row.bio = Some(bio.clone());
    }

    // Handle status_message update
    if let Some(ref sm) = req.status_message {
        if sm.len() > 128 {
            return Err(ApiError::BadRequest("status message must be 128 characters or less".into()));
        }
        user_row.status_message = if sm.is_empty() { None } else { Some(sm.clone()) };
    }

    let has_changes = req.username.is_some() || req.email.is_some()
        || req.bio.is_some() || req.status_message.is_some();

    if has_changes {
        sqlx::query(
            "UPDATE users SET username = $1, email = $2, bio = $3, status_message = $4, updated_at = NOW() WHERE id = $5"
        )
        .bind(&user_row.username)
        .bind(&user_row.email)
        .bind(&user_row.bio)
        .bind(&user_row.status_message)
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
        bio: user_row.bio,
        status_message: user_row.status_message,
        totp_enabled: user_row.totp_enabled,
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
    fn test_update_request() {
        let partial = UpdateMeRequest {
            username: Some("new".into()),
            email: None,
            bio: None,
            status_message: None,
        };
        assert!(partial.username.is_some());
        assert!(partial.email.is_none());

        let empty = UpdateMeRequest {
            username: None,
            email: None,
            bio: None,
            status_message: None,
        };
        assert!(empty.username.is_none());
    }
}
