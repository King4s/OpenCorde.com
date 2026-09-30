//! # Route: Messages
//! Message CRUD operations and typing indicator for channels.
//!
//! ## Endpoints
//! - POST /api/v1/channels/{channel_id}/messages — Send message to channel
//! - GET /api/v1/channels/{channel_id}/messages — List channel messages (paginated)
//! - PATCH /api/v1/messages/{id} — Edit message (author only)
//! - DELETE /api/v1/messages/{id} — Delete message (author only)
//! - POST /api/v1/messages/{id} — Edit/delete message
//! - POST /api/v1/channels/{channel_id}/typing — Typing indicator
//! - POST /api/v1/messages/{id}/forward — Forward message to another channel
//! - `handlers` — Route handler functions and router
//! - `send_list` — Send and list message handlers
//! - `edit_delete` — Edit, delete, and typing indicator handlers
//! - `types` — Request/response types
//! - `validation` — Input validation
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db::repos::message_repo (database operations)
//! - opencorde_core::Snowflake (ID generation)
//! - crate::middleware::auth::AuthUser (authentication)
//! - crate::AppState (application state)

//! - POST /api/v1/messages/{id}/forward — Forward message to another channel

mod edit_delete;
mod forward;
mod handlers;
mod helpers;
mod list;
mod notifications;
mod send;
pub mod send_list;
pub mod types;
pub mod validation;
pub use handlers::router;
pub use send_list::message_row_to_response;
pub use types::{
    EditMessageRequest, ForwardMessageRequest, MessageQuery, MessageResponse, SendMessageRequest,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let _router = router();
        // Verify all route modules can be composed without panic
    }
}
