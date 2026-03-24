//! # Forum Handlers
//! HTTP request handlers for forum endpoints.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use opencorde_core::snowflake::SnowflakeGenerator;
use opencorde_db::repos::forum_repo;
use tracing::instrument;

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};
use crate::routes::helpers;

use super::types::{CreatePostRequest, CreateReplyRequest, ListPostsQuery, PostDetailResponse, PostResponse, ReplyResponse};
use super::auth_check;

/// Create a forum post in a channel. Verifies channel exists and is forum type (2).
#[instrument(skip(state, auth, body), fields(user_id = %auth.user_id))]
pub async fn create_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
    Json(body): Json<CreatePostRequest>,
) -> Result<(StatusCode, Json<PostResponse>), ApiError> {
    tracing::info!(channel_id = %channel_id, "creating forum post");

    if body.title.is_empty() || body.title.len() > 200 {
        return Err(ApiError::BadRequest("title must be 1-200 characters".into()));
    }
    if body.content.is_empty() {
        return Err(ApiError::BadRequest("content cannot be empty".into()));
    }

    let channel_id_sf = helpers::parse_snowflake(&channel_id)?;

    // Verify channel exists and is forum type (2)
    let channel_type: (i16,) = sqlx::query_as("SELECT channel_type FROM channels WHERE id = $1")
        .bind(channel_id_sf.as_i64())
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("channel not found".into()))?;

    if channel_type.0 != 2 {
        return Err(ApiError::BadRequest("channel is not a forum channel".into()));
    }

    let mut generator = SnowflakeGenerator::new(3, 0);
    let post_id = generator.next_id();
    let row = forum_repo::create_post(
        &state.db,
        post_id,
        channel_id_sf,
        auth.user_id,
        &body.title,
        &body.content,
    )
    .await
    .map_err(ApiError::Database)?;

    let response = PostResponse {
        id: row.id.to_string(),
        channel_id: row.channel_id.to_string(),
        author_id: row.author_id.to_string(),
        author_username: row.author_username,
        title: row.title,
        content: row.content,
        reply_count: row.reply_count,
        pinned: row.pinned,
        created_at: row.created_at,
        last_reply_at: row.last_reply_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// List posts in a channel with pagination (limit, default 20, max 100).
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn list_posts(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
    Query(params): Query<ListPostsQuery>,
) -> Result<Json<Vec<PostResponse>>, ApiError> {
    tracing::info!(channel_id = %channel_id, limit = params.limit, "listing forum posts");

    let channel_id_sf = helpers::parse_snowflake(&channel_id)?;
    let limit = params.limit.clamp(1, 100);

    let rows = forum_repo::list_posts(&state.db, channel_id_sf, limit)
        .await
        .map_err(ApiError::Database)?;

    let posts = rows
        .into_iter()
        .map(|r| PostResponse {
            id: r.id.to_string(),
            channel_id: r.channel_id.to_string(),
            author_id: r.author_id.to_string(),
            author_username: r.author_username,
            title: r.title,
            content: r.content,
            reply_count: r.reply_count,
            pinned: r.pinned,
            created_at: r.created_at,
            last_reply_at: r.last_reply_at,
        })
        .collect();

    Ok(Json(posts))
}

/// Get a post with all its replies.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn get_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(post_id): Path<String>,
) -> Result<Json<PostDetailResponse>, ApiError> {
    tracing::info!(post_id = %post_id, "getting forum post detail");

    let post_id_sf = helpers::parse_snowflake(&post_id)?;

    let post = forum_repo::get_post(&state.db, post_id_sf)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("post not found".into()))?;

    let replies = forum_repo::list_replies(&state.db, post_id_sf)
        .await
        .map_err(ApiError::Database)?;

    let post_response = PostResponse {
        id: post.id.to_string(),
        channel_id: post.channel_id.to_string(),
        author_id: post.author_id.to_string(),
        author_username: post.author_username,
        title: post.title,
        content: post.content,
        reply_count: post.reply_count,
        pinned: post.pinned,
        created_at: post.created_at,
        last_reply_at: post.last_reply_at,
    };

    let reply_responses = replies
        .into_iter()
        .map(|r| ReplyResponse {
            id: r.id.to_string(),
            post_id: r.post_id.to_string(),
            author_id: r.author_id.to_string(),
            author_username: r.author_username,
            content: r.content,
            created_at: r.created_at,
        })
        .collect();

    Ok(Json(PostDetailResponse {
        post: post_response,
        replies: reply_responses,
    }))
}

/// Delete a post. Author or server owner can delete.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn delete_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(post_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(post_id = %post_id, "deleting forum post");

    let post_id_sf = helpers::parse_snowflake(&post_id)?;
    let post = forum_repo::get_post(&state.db, post_id_sf)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("post not found".into()))?;

    auth_check::check_post_author_or_owner(&state, &post, &auth).await?;

    forum_repo::delete_post(&state.db, post_id_sf)
        .await
        .map_err(ApiError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Add a reply to a post.
#[instrument(skip(state, auth, body), fields(user_id = %auth.user_id))]
pub async fn create_reply(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(post_id): Path<String>,
    Json(body): Json<CreateReplyRequest>,
) -> Result<(StatusCode, Json<ReplyResponse>), ApiError> {
    tracing::info!(post_id = %post_id, "creating forum reply");

    if body.content.is_empty() {
        return Err(ApiError::BadRequest("content cannot be empty".into()));
    }

    let post_id_sf = helpers::parse_snowflake(&post_id)?;

    forum_repo::get_post(&state.db, post_id_sf)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("post not found".into()))?;

    let mut generator = SnowflakeGenerator::new(3, 0);
    let reply_id = generator.next_id();
    let row = forum_repo::create_reply(
        &state.db,
        reply_id,
        post_id_sf,
        auth.user_id,
        &body.content,
    )
    .await
    .map_err(ApiError::Database)?;

    let response = ReplyResponse {
        id: row.id.to_string(),
        post_id: row.post_id.to_string(),
        author_id: row.author_id.to_string(),
        author_username: row.author_username,
        content: row.content,
        created_at: row.created_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Delete a reply. Author or server owner can delete.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn delete_reply(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(reply_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(reply_id = %reply_id, "deleting forum reply");

    let reply_id_sf = helpers::parse_snowflake(&reply_id)?;

    let reply: Option<(i64, i64)> = sqlx::query_as("SELECT author_id, post_id FROM forum_replies WHERE id = $1")
        .bind(reply_id_sf.as_i64())
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::Database)?;

    let (author_id, post_id) = reply.ok_or_else(|| ApiError::NotFound("reply not found".into()))?;

    auth_check::check_reply_author_or_owner(&state, author_id, post_id, &auth).await?;

    forum_repo::delete_reply(&state.db, reply_id_sf)
        .await
        .map_err(ApiError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}
