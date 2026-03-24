//! # WebSocket Main Loop
//! Heartbeat generation, event dispatch, and client message handling.

use axum::extract::ws::Message;
use futures::{SinkExt, StreamExt};
use std::collections::HashSet;
use std::time::Duration;
use tokio::time::interval;

use crate::ws::dispatch::should_dispatch;

pub const HEARTBEAT_INTERVAL_SECS: u64 = 30;

/// Run the main event loop for a connected WebSocket client.
///
/// Handles:
/// - Periodic heartbeats
/// - Event dispatch from broadcast channel (filtered by accessible channels)
/// - Incoming client messages (HeartbeatAck, etc.)
/// - Connection closure and offline presence broadcast
pub async fn run_main_loop(
    mut sender: futures::stream::SplitSink<
        axum::extract::ws::WebSocket,
        Message,
    >,
    mut receiver: futures::stream::SplitStream<axum::extract::ws::WebSocket>,
    mut event_rx: tokio::sync::broadcast::Receiver<serde_json::Value>,
    user_id_str: String,
    accessible_channels: HashSet<i64>,
    event_tx: tokio::sync::broadcast::Sender<serde_json::Value>,
) {
    // Main event loop: heartbeats + event fan-out + client messages
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
                        let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("?");
                        let dispatch = should_dispatch(&event, &accessible_channels);
                        tracing::debug!(user_id = %user_id_str, event_type, dispatch, "event received from broadcast");
                        // Only forward events for channels this user can access
                        if dispatch
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

    // Broadcast this user's offline presence to all remaining connected clients
    let presence_offline = serde_json::json!({
        "type": "PresenceUpdate",
        "data": { "user_id": user_id_str, "online": false }
    });
    if event_tx.send(presence_offline).is_err() {
        tracing::debug!(user_id = %user_id_str, "no other WS subscribers for offline PresenceUpdate");
    }

    tracing::info!(user_id = %user_id_str, "connection closed");
}
