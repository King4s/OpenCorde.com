//! # Route: Friends
//! Friend requests, acceptance, blocking, and friend list.
//!
//! ## Endpoints
//! - POST /api/v1/friends/request — Send friend request (body: { user_id })
//! - GET /api/v1/friends — List friends
//! - GET /api/v1/friends/pending — List pending requests (incoming + outgoing)
//! - PUT /api/v1/friends/{relationship_id}/accept — Accept friend request
//! - DELETE /api/v1/friends/{relationship_id} — Remove friend or decline request
//! - POST /api/v1/friends/block — Block user (body: { user_id })
//! - GET /api/v1/users/search?q={query} — Search users by username

use axum::{
    Router,
    routing::{delete, get, post, put},
};

use crate::AppState;

mod accept;
mod block;
mod list;
mod pending;
mod remove;
mod request;
mod search;
mod types;

pub use types::{PendingResponse, RelationshipResponse, UserIdRequest, UserSearchResult};

/// Build the friends router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/friends", get(list::list_friends))
        .route("/api/v1/friends/request", post(request::send_request))
        .route("/api/v1/friends/pending", get(pending::list_pending))
        .route("/api/v1/friends/block", post(block::block_user))
        .route(
            "/api/v1/friends/{relationship_id}/accept",
            put(accept::accept_request),
        )
        .route(
            "/api/v1/friends/{relationship_id}",
            delete(remove::remove_relationship),
        )
        .route("/api/v1/users/search", get(search::search_users))
}
