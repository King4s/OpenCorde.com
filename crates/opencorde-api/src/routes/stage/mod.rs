//! # Route: Stage Channels
//! Stage channel session and participant management.
//!
//! ## Endpoints
//! - POST /api/v1/channels/{channel_id}/stage/start — Start session (server owner)
//! - GET|DELETE /api/v1/channels/{channel_id}/stage — Get detail / End session
//! - POST /api/v1/channels/{channel_id}/stage/join — Join as audience
//! - DELETE /api/v1/channels/{channel_id}/stage/leave — Leave stage
//! - POST /api/v1/channels/{channel_id}/stage/hand — Raise/lower hand
//! - PATCH /api/v1/channels/{channel_id}/stage/speakers/{user_id} — Promote/demote
//!
//! ## Depends On
//! - axum, opencorde_db::repos::stage_repo, crate::AppState

use axum::{Router, routing};
use crate::AppState;

pub mod types;
pub mod handlers;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/channels/{channel_id}/stage/start",
            routing::post(handlers::start_stage),
        )
        .route(
            "/api/v1/channels/{channel_id}/stage",
            routing::get(handlers::get_stage).delete(handlers::end_stage),
        )
        .route(
            "/api/v1/channels/{channel_id}/stage/join",
            routing::post(handlers::join_stage),
        )
        .route(
            "/api/v1/channels/{channel_id}/stage/leave",
            routing::delete(handlers::leave_stage),
        )
        .route(
            "/api/v1/channels/{channel_id}/stage/hand",
            routing::post(handlers::toggle_hand),
        )
        .route(
            "/api/v1/channels/{channel_id}/stage/speakers/{user_id}",
            routing::patch(handlers::set_speaker),
        )
}
