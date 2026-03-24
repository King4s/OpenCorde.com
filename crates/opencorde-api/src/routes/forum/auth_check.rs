//! # Forum Authorization Checks
//! Helpers for verifying user permissions on forum posts and replies.

use opencorde_db::repos::forum_repo;

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};

/// Check if user is author or server owner of a post.
pub async fn check_post_author_or_owner(
    state: &AppState,
    post: &forum_repo::ForumPostRow,
    auth: &AuthUser,
) -> Result<(), ApiError> {
    if post.author_id == auth.user_id.as_i64() {
        return Ok(());
    }

    let channel: (i64,) = sqlx::query_as("SELECT server_id FROM channels WHERE id = $1")
        .bind(post.channel_id)
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("channel not found".into()))?;

    let owner: (i64,) = sqlx::query_as("SELECT owner_id FROM servers WHERE id = $1")
        .bind(channel.0)
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;

    if owner.0 != auth.user_id.as_i64() {
        return Err(ApiError::Forbidden);
    }

    Ok(())
}

/// Check if user is author or server owner of a reply.
pub async fn check_reply_author_or_owner(
    state: &AppState,
    author_id: i64,
    post_id: i64,
    auth: &AuthUser,
) -> Result<(), ApiError> {
    if author_id == auth.user_id.as_i64() {
        return Ok(());
    }

    let channel: (i64,) = sqlx::query_as(
        "SELECT c.server_id FROM channels c \
         JOIN forum_posts fp ON c.id = fp.channel_id \
         WHERE fp.id = $1"
    )
    .bind(post_id)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("post not found".into()))?;

    let owner: (i64,) = sqlx::query_as("SELECT owner_id FROM servers WHERE id = $1")
        .bind(channel.0)
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;

    if owner.0 != auth.user_id.as_i64() {
        return Err(ApiError::Forbidden);
    }

    Ok(())
}
