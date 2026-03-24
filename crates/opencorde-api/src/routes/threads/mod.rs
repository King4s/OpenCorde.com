//! # Route: Threads
//! Thread creation and message retrieval for sub-conversations.
//!
//! ## Endpoints
//! - POST /api/v1/channels/{channel_id}/messages/{message_id}/thread — Create thread from message
//! - GET /api/v1/channels/{channel_id}/threads — List threads in channel
//! - GET /api/v1/threads/{thread_id} — Get a single thread
//! - GET /api/v1/threads/{thread_id}/messages — List messages in thread
//! - POST /api/v1/threads/{thread_id}/messages — Send message in thread
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db::repos (database operations)
//! - opencorde_core::Snowflake (ID generation)
//! - crate::middleware::auth::AuthUser (authentication)
//! - crate::AppState (application state)

mod handlers;
mod types;

pub use types::{CreateThreadRequest, SendThreadMessageRequest, ThreadResponse};
pub use handlers::router;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let _router = router();
        // Verify all route modules can be composed without panic
    }
}
