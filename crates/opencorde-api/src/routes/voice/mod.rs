//! # Route: Voice
//! Voice channel management and LiveKit token generation.
//!
//! ## Endpoints
//! - POST /api/v1/voice/join — Join a voice channel, receive LiveKit token
//! - POST /api/v1/voice/leave — Leave current voice channel
//! - PATCH /api/v1/voice/state — Update voice state (mute/deafen)
//! - GET /api/v1/voice/participants/{channel_id} — List channel participants
//! - POST /api/v1/livekit/token — Get fresh LiveKit access token
//!
//! ## Features
//! - Authentication required (AuthUser)
//! - Channel existence and type validation
//! - LiveKit JWT token generation with proper claims
//! - Comprehensive structured logging
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db::repos::voice_state_repo (voice CRUD)
//! - crate::middleware::auth::AuthUser (authentication)
//! - crate::AppState (database + config)
//! - jsonwebtoken (JWT generation)

pub mod handlers;
pub mod livekit;
pub mod types;

use axum::Router;

use crate::AppState;

/// Build the voice router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/voice/join",
            axum::routing::post(handlers::join_voice),
        )
        .route(
            "/api/v1/voice/leave",
            axum::routing::post(handlers::leave_voice),
        )
        .route(
            "/api/v1/voice/state",
            axum::routing::patch(handlers::update_voice_state),
        )
        .route(
            "/api/v1/voice/participants/{channel_id}",
            axum::routing::get(handlers::get_participants),
        )
        .route(
            "/api/v1/livekit/token",
            axum::routing::post(handlers::get_livekit_token),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_router_creation() {
        let _router = router();
        // Verify all routes can be composed without panic
    }
}
