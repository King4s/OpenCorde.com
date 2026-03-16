//! # Message Route Handlers
//! HTTP route handlers and router for message CRUD and typing indicator endpoints.
//!
//! ## Endpoints
//! - POST /api/v1/channels/{channel_id}/messages — Send message
//! - GET /api/v1/channels/{channel_id}/messages — List messages (cursor pagination)
//! - PATCH /api/v1/messages/{id} — Edit message (author only)
//! - DELETE /api/v1/messages/{id} — Delete message (author only)
//! - POST /api/v1/channels/{channel_id}/typing — Typing indicator
//!
//! ## Depends On
//! - axum (web framework)
//! - crate::AppState (application state)

use axum::{
    Router,
    routing::{patch, post},
};

use crate::AppState;

use super::{edit_delete, send_list};

/// Build the messages router with all endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/channels/{channel_id}/messages",
            post(send_list::send_message).get(send_list::list_messages),
        )
        .route(
            "/api/v1/messages/{id}",
            patch(edit_delete::edit_message).delete(edit_delete::delete_message),
        )
        .route(
            "/api/v1/channels/{channel_id}/typing",
            post(edit_delete::typing_indicator),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let _router = router();
        // Verify all route modules can be composed without panic
    }
}
