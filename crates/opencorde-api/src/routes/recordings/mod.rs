//! # Route: Recordings
//! LiveKit Egress-based call recording endpoints.
//!
//! ## Endpoints
//! - POST /api/v1/channels/{id}/recording/start — Start RoomCompositeEgress
//! - POST /api/v1/channels/{id}/recording/stop  — Stop active Egress job
//! - GET  /api/v1/channels/{id}/recordings      — List recordings for channel
//!
//! ## Modules
//! - `handlers` — HTTP handler functions
//! - `types`    — Wire types and DB row structs
//!
//! ## Depends On
//! - axum (routing)
//! - reqwest (LiveKit Egress HTTP calls)
//! - crate::AppState, crate::error::ApiError
//! - crate::routes::voice::livekit (token generation)

pub mod handlers;
pub mod types;

use axum::Router;
use axum::routing::{get, post};

use crate::AppState;

/// Build the recordings router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/channels/{id}/recording/start",
            post(handlers::start_recording),
        )
        .route(
            "/api/v1/channels/{id}/recording/stop",
            post(handlers::stop_recording),
        )
        .route(
            "/api/v1/channels/{id}/recordings",
            get(handlers::list_recordings),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recordings_router_creation() {
        let _router = router();
    }
}
