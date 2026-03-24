//! # Route: Forum Channels
//! Post/reply model for forum-type channels.
//!
//! ## Endpoints
//! - POST /api/v1/channels/{channel_id}/posts — Create post
//! - GET /api/v1/channels/{channel_id}/posts — List posts (query: limit=20)
//! - GET /api/v1/posts/{post_id} — Get post + replies
//! - DELETE /api/v1/posts/{post_id} — Delete post (author or owner)
//! - POST /api/v1/posts/{post_id}/replies — Add reply
//! - DELETE /api/v1/replies/{reply_id} — Delete reply (author or owner)
//!
//! ## Depends On
//! - opencorde_db::repos::forum_repo
//! - crate::routes::helpers

mod auth_check;
mod handlers;
mod types;

use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::AppState;

pub use types::{CreatePostRequest, CreateReplyRequest, PostDetailResponse, PostResponse, ReplyResponse};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/channels/{channel_id}/posts", post(handlers::create_post).get(handlers::list_posts))
        .route("/api/v1/posts/{post_id}", get(handlers::get_post).delete(handlers::delete_post))
        .route("/api/v1/posts/{post_id}/replies", post(handlers::create_reply))
        .route("/api/v1/replies/{reply_id}", delete(handlers::delete_reply))
}
