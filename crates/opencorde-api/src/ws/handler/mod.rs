//! # WebSocket Handler
//! WebSocket connection lifecycle management and event streaming.
//!
//! ## Flow
//! 1. Client connects via `/api/v1/gateway`
//! 2. Server sends HELLO with heartbeat_interval
//! 3. Client sends IDENTIFY with JWT token
//! 4. Server validates token, sends READY with user info
//! 5. Server subscribes to event broadcast for channels the user can access
//! 6. Main loop: heartbeats + event fan-out to this client
//!
//! ## Depends On
//! - axum::extract::ws (WebSocket primitives)
//! - futures (SinkExt, StreamExt)
//! - crate::jwt (token validation)
//! - crate::AppState (shared state)
//! - serde_json (event serialization)
//! - opencorde_db::repos::channel_repo (user's accessible channels)

mod lifecycle;
mod main_loop;

pub use lifecycle::handle_connection;

use axum::{
    Router,
    extract::{
        State,
        ws::WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
};
use tracing::instrument;

use crate::AppState;

pub use main_loop::run_main_loop;

/// Build the WebSocket gateway router.
pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/gateway", get(ws_upgrade))
}

/// Handle WebSocket upgrade requests.
#[instrument(skip(ws, state))]
async fn ws_upgrade(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    tracing::info!("WebSocket upgrade requested");
    ws.on_upgrade(move |socket| handle_connection(socket, state))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let _r = router();
    }

    #[test]
    fn test_constants() {
        assert_eq!(lifecycle::IDENTIFY_TIMEOUT_SECS, 10);
        assert_eq!(main_loop::HEARTBEAT_INTERVAL_SECS, 30);
    }

    #[test]
    fn test_should_dispatch_delegates_to_dispatch_module() {
        use crate::ws::dispatch::should_dispatch;
        use std::collections::HashSet;

        // Verify handler uses dispatch module's should_dispatch
        let mut channels = HashSet::new();
        channels.insert(12345_i64);
        let event = serde_json::json!({
            "type": "MessageCreate",
            "data": { "message": { "channel_id": "12345", "content": "hello" } }
        });
        assert!(should_dispatch(&event, &channels));
        // PresenceUpdate always dispatched
        let presence = serde_json::json!({"type":"PresenceUpdate","data":{}});
        assert!(should_dispatch(&presence, &channels));
        // ChannelAck always dispatched
        let ack = serde_json::json!({"type":"ChannelAck","data":{}});
        assert!(should_dispatch(&ack, &channels));
    }
}
