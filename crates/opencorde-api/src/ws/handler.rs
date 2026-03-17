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
use opencorde_core::Snowflake;
use opencorde_db::repos::channel_repo;
use std::collections::HashSet;
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
/// 5. Subscribe to broadcast channel, load accessible channel IDs
/// 6. Main loop: heartbeats + event dispatch + client messages
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

    let user_id_str = claims.sub.clone();
    let username = claims.username.clone();
    tracing::info!(user_id = %user_id_str, username = %username, "user authenticated");

    // 4. Send READY with user info
    let ready = serde_json::json!({
        "type": "Ready",
        "data": {
            "user": {
                "id": user_id_str,
                "username": username
            },
            "servers": []
        }
    });

    if let Err(e) = sender.send(Message::Text(ready.to_string().into())).await {
        tracing::warn!("failed to send READY: {}", e);
        return;
    }
    tracing::debug!(user_id = %user_id_str, "sent READY");

    // 5. Subscribe to event broadcast and load accessible channel IDs
    let mut event_rx = state.event_tx.subscribe();

    let user_id_i64: i64 = match user_id_str.parse() {
        Ok(v) => v,
        Err(_) => {
            tracing::error!(user_id = %user_id_str, "failed to parse user ID as i64");
            return;
        }
    };
    let user_snowflake = Snowflake::new(user_id_i64);

    let accessible_channels: HashSet<i64> = channel_repo::list_ids_by_user(&state.db, user_snowflake)
        .await
        .unwrap_or_default()
        .into_iter()
        .collect();

    tracing::debug!(
        user_id = %user_id_str,
        channel_count = accessible_channels.len(),
        "loaded accessible channels for WebSocket session"
    );

    // 6. Main event loop: heartbeats + event fan-out + client messages
    // MissedTickBehavior::Skip avoids burst on resume; Delay skips the immediate first tick.
    let mut heartbeat_timer = {
        let mut t = interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
        t.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // Consume the immediately-firing first tick so clients don't get a Heartbeat on connect.
        t.tick().await;
        t
    };

    loop {
        tokio::select! {
            _ = heartbeat_timer.tick() => {
                let heartbeat = serde_json::json!({"type": "Heartbeat"});
                if let Err(e) = sender.send(Message::Text(heartbeat.to_string().into())).await {
                    tracing::info!(user_id = %user_id_str, "connection lost during heartbeat: {}", e);
                    break;
                }
                tracing::trace!(user_id = %user_id_str, "heartbeat sent");
            }

            result = event_rx.recv() => {
                match result {
                    Ok(event) => {
                        // Only forward events for channels this user can access
                        if should_dispatch(&event, &accessible_channels)
                            && let Err(e) = sender.send(Message::Text(event.to_string().into())).await
                        {
                            tracing::info!(user_id = %user_id_str, "connection lost sending event: {}", e);
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(user_id = %user_id_str, skipped = n, "broadcast receiver lagged");
                        // Continue — we just missed some events, not fatal
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!(user_id = %user_id_str, "broadcast channel closed");
                        break;
                    }
                }
            }

            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(event) = serde_json::from_str::<serde_json::Value>(&text)
                            && let Some(event_type) = event.get("type").and_then(|v| v.as_str())
                        {
                            match event_type {
                                "HeartbeatAck" => {
                                    tracing::trace!(user_id = %user_id_str, "heartbeat ack received");
                                }
                                _ => {
                                    tracing::debug!(user_id = %user_id_str, event_type, "received client event");
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!(user_id = %user_id_str, "WebSocket closed");
                        break;
                    }
                    Some(Ok(Message::Binary(_))) => {
                        tracing::debug!(user_id = %user_id_str, "ignoring binary message");
                    }
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {
                        // handled automatically by axum
                    }
                    Some(Err(e)) => {
                        tracing::warn!(user_id = %user_id_str, "WebSocket error: {}", e);
                        break;
                    }
                }
            }
        }
    }

    tracing::info!(user_id = %user_id_str, "connection closed");
}

/// Check if an event should be dispatched to this WebSocket session.
///
/// For MessageCreate events, checks that the message's channel_id is in the
/// user's accessible channels set.
fn should_dispatch(event: &serde_json::Value, accessible_channels: &HashSet<i64>) -> bool {
    let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match event_type {
        "MessageCreate" => {
            let channel_id = event
                .get("data")
                .and_then(|d| d.get("message"))
                .and_then(|m| m.get("channel_id"))
                .and_then(|c| c.as_str())
                .and_then(|c| c.parse::<i64>().ok());

            channel_id.map(|id| accessible_channels.contains(&id)).unwrap_or(false)
        }
        // Future event types can be added here
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_router_creation() {
        let _r = router();
    }

    #[test]
    fn test_constants() {
        assert_eq!(IDENTIFY_TIMEOUT_SECS, 10);
        assert_eq!(HEARTBEAT_INTERVAL_SECS, 30);
    }

    #[test]
    fn test_should_dispatch_message_create() {
        let mut channels = HashSet::new();
        channels.insert(12345_i64);

        let event = serde_json::json!({
            "type": "MessageCreate",
            "data": { "message": { "channel_id": "12345", "content": "hello" } }
        });
        assert!(should_dispatch(&event, &channels));

        let event_other = serde_json::json!({
            "type": "MessageCreate",
            "data": { "message": { "channel_id": "99999", "content": "hello" } }
        });
        assert!(!should_dispatch(&event_other, &channels));
    }

    #[test]
    fn test_should_dispatch_unknown_type() {
        let channels = HashSet::new();
        let event = serde_json::json!({"type": "Unknown", "data": {}});
        assert!(!should_dispatch(&event, &channels));
    }
}
