//! # WebSocket Handler
//! WebSocket connection lifecycle management and event streaming.
//!
//! ## Flow
//! 1. Client connects via `/api/v1/gateway`
//! 2. Server sends HELLO with heartbeat_interval
//! 3. Client sends IDENTIFY with JWT token
//! 4. Server validates token, sends READY with user info
//! 5. Server sends periodic HEARTBEAT, client responds HEARTBEAT_ACK
//! 6. Server pushes events (MESSAGE_CREATE, etc.) to connected clients
//!
//! ## Depends On
//! - axum::extract::ws (WebSocket primitives)
//! - futures (SinkExt, StreamExt)
//! - crate::jwt (token validation)
//! - crate::AppState (shared state)
//! - serde_json (event serialization)

use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::get,
};
use futures::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::time::interval;
use tracing::instrument;

use crate::AppState;

const IDENTIFY_TIMEOUT_SECS: u64 = 10;
const HEARTBEAT_INTERVAL_SECS: u64 = 30;

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

/// Manage a single WebSocket connection lifecycle.
///
/// 1. Send HELLO with heartbeat interval
/// 2. Wait for IDENTIFY (with timeout)
/// 3. Validate JWT token
/// 4. Send READY with user info
/// 5. Main loop: heartbeats + message handling
#[instrument(skip(socket, state))]
async fn handle_connection(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // 1. Send HELLO
    let hello = serde_json::json!({
        "type": "Hello",
        "data": {
            "heartbeat_interval": HEARTBEAT_INTERVAL_SECS * 1000  // milliseconds
        }
    });

    if let Err(e) = sender.send(Message::Text(hello.to_string().into())).await {
        tracing::warn!("failed to send HELLO: {}", e);
        return;
    }
    tracing::debug!("sent HELLO with heartbeat_interval");

    // 2. Wait for IDENTIFY (with timeout)
    let identify_timeout = Duration::from_secs(IDENTIFY_TIMEOUT_SECS);
    let identify_result = tokio::time::timeout(identify_timeout, async {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg
                && let Ok(event) = serde_json::from_str::<serde_json::Value>(&text)
                && event.get("type").and_then(|v| v.as_str()) == Some("Identify")
            {
                return Some(event);
            }
        }
        None
    })
    .await;

    let identify = match identify_result {
        Ok(Some(event)) => event,
        Ok(None) => {
            tracing::warn!("IDENTIFY not received before receiver closed");
            let _ = sender.close().await;
            return;
        }
        Err(_) => {
            tracing::warn!(timeout_secs = IDENTIFY_TIMEOUT_SECS, "IDENTIFY timeout");
            let _ = sender.close().await;
            return;
        }
    };

    // 3. Extract and validate token
    let token = identify
        .get("data")
        .and_then(|d| d.get("token"))
        .and_then(|t| t.as_str())
        .unwrap_or("");

    let claims = match crate::jwt::validate_access_token(token, &state.config.jwt_secret) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("token validation failed: {}", e);
            let error_msg = serde_json::json!({
                "type": "Error",
                "data": { "message": "invalid token" }
            });
            let _ = sender
                .send(Message::Text(error_msg.to_string().into()))
                .await;
            let _ = sender.close().await;
            return;
        }
    };

    let user_id = claims.sub.clone();
    let username = claims.username.clone();
    tracing::info!(user_id = %user_id, username = %username, "user authenticated");

    // 4. Send READY with user info
    let ready = serde_json::json!({
        "type": "Ready",
        "data": {
            "user": {
                "id": user_id,
                "username": username
            },
            "servers": []  // TODO: fetch user's servers
        }
    });

    if let Err(e) = sender.send(Message::Text(ready.to_string().into())).await {
        tracing::warn!("failed to send READY: {}", e);
        return;
    }
    tracing::debug!(user_id = %user_id, "sent READY");

    // 5. Main event loop: heartbeats + message handling
    let mut heartbeat_timer = interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));

    loop {
        tokio::select! {
            _ = heartbeat_timer.tick() => {
                let heartbeat = serde_json::json!({"type": "Heartbeat"});
                if let Err(e) = sender.send(Message::Text(heartbeat.to_string().into())).await {
                    tracing::info!(user_id = %user_id, "connection lost during heartbeat: {}", e);
                    break;
                }
                tracing::trace!(user_id = %user_id, "heartbeat sent");
            }

            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(event) = serde_json::from_str::<serde_json::Value>(&text)
                            && let Some(event_type) = event.get("type").and_then(|v| v.as_str())
                        {
                            match event_type {
                                "HeartbeatAck" => {
                                    tracing::trace!(user_id = %user_id, "heartbeat ack received");
                                }
                                _ => {
                                    tracing::debug!(user_id = %user_id, event_type, "received event");
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!(user_id = %user_id, "WebSocket closed");
                        break;
                    }
                    Some(Ok(Message::Binary(_))) => {
                        tracing::debug!(user_id = %user_id, "ignoring binary message");
                    }
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {
                        // handled automatically by axum
                    }
                    Some(Err(e)) => {
                        tracing::warn!(user_id = %user_id, "WebSocket error: {}", e);
                        break;
                    }
                }
            }
        }
    }

    tracing::info!(user_id = %user_id, "connection closed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let _r = router();
        // Verify router can be created without panic
    }

    #[test]
    fn test_constants() {
        assert_eq!(IDENTIFY_TIMEOUT_SECS, 10);
        assert_eq!(HEARTBEAT_INTERVAL_SECS, 30);
    }
}
