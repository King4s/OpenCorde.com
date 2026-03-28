//! # WebSocket Lifecycle
//! Connection setup, authentication, and teardown.

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use opencorde_core::Snowflake;
use opencorde_db::repos::{channel_repo, server_repo};
use std::collections::HashSet;
use std::time::Duration;
use tracing::instrument;

use crate::AppState;
use super::main_loop::run_main_loop;

pub const IDENTIFY_TIMEOUT_SECS: u64 = 10;
pub const HEARTBEAT_INTERVAL_SECS: u64 = 30;

/// Manage a single WebSocket connection lifecycle.
///
/// 1. Send HELLO with heartbeat interval
/// 2. Wait for IDENTIFY (with timeout)
/// 3. Validate JWT token
/// 4. Send READY with user info
/// 5. Subscribe to broadcast channel, load accessible channel IDs
/// 6. Main loop: heartbeats + event dispatch + client messages
#[instrument(skip(socket, state))]
pub async fn handle_connection(socket: WebSocket, state: AppState) {
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

    // 5. Subscribe to event broadcast BEFORE broadcasting presence so this client
    //    can receive the PresenceUpdate it triggers (useful for testing; other clients
    //    receive it via the shared broadcast channel).
    let event_rx = state.event_tx.subscribe();

    // Broadcast this user's online presence to all connected clients
    let presence_online = serde_json::json!({
        "type": "PresenceUpdate",
        "data": { "user_id": user_id_str, "online": true }
    });
    if state.event_tx.send(presence_online).is_err() {
        tracing::debug!(user_id = %user_id_str, "no other WS subscribers for online PresenceUpdate");
    }

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

    let member_server_ids: HashSet<i64> = server_repo::list_by_user(&state.db, user_snowflake)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.id)
        .collect();

    tracing::debug!(
        user_id = %user_id_str,
        channel_count = accessible_channels.len(),
        server_count = member_server_ids.len(),
        "loaded accessible channels and servers for WebSocket session"
    );

    // 6. Run main event loop
    run_main_loop(
        sender,
        receiver,
        event_rx,
        user_id_str.clone(),
        accessible_channels,
        member_server_ids,
        (*state.event_tx).clone(),
    )
    .await;
}
