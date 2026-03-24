//! # Route: Direct Messages
//! DM channel management and messaging endpoints.
//!
//! ## Endpoints
//! - GET /api/v1/users/@me/channels — List DM channels for current user
//! - POST /api/v1/users/@me/channels — Open DM with a user
//! - GET /api/v1/channels/@dms/{dm_id}/messages — List DM messages
//! - POST /api/v1/channels/@dms/{dm_id}/messages — Send DM message
//!
//! ## Features
//! - Get-or-create DM channel logic
//! - Cursor-based pagination for messages (before cursor)
//! - Membership validation before read/write operations
//! - Real-time WS event broadcasting on new messages
//! - Comprehensive structured logging
//!
//! ## Modules
//! - `types` — Request/response data types
//! - `handlers` — HTTP request handlers
//!
//! ## Depends On
//! - axum (web framework)
//! - crate::AppState (database + event broadcasting)

mod handlers;
pub mod types;

use axum::{
    routing::get,
    Router,
};

use crate::AppState;

/// Build the DMs router with all endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/users/@me/channels",
            get(handlers::list_dms).post(handlers::open_dm),
        )
        .route(
            "/api/v1/channels/@dms/{dm_id}/messages",
            get(handlers::list_dm_messages).post(handlers::send_dm_message),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let _router = router();
    }
}
