//! # Event Dispatch Filtering
//! Determines which WebSocket events should be delivered to each connected client
//! based on channel accessibility and event type.
//!
//! ## Rationale
//! Users should only receive events for channels they can access. This module
//! centralizes the dispatch logic to ensure consistent filtering across all event types.
//!
//! ## Event Types
//! - MessageCreate/Update/Delete: dispatched if user can access the message's channel
//! - TypingStart: dispatched if user can access the channel
//! - ReactionAdd/ReactionRemove: dispatched if user can access the message's channel
//! - ChannelCreate/Update/Delete: dispatched if user is a member of the server
//! - RoleCreate/Update/Delete: dispatched if user is a member of the server
//! - MemberUpdate: dispatched if user is a member of the server
//! - ServerUpdate: dispatched if user is a member of the server
//! - ChannelAck: dispatched to all connected users (client filters by user_id)
//! - PresenceUpdate/MemberJoin/MemberLeave: dispatched to all connected users

use std::collections::HashSet;

/// Check if an event should be dispatched to a WebSocket session.
///
/// Different event types have different access requirements:
/// - MessageCreate/Update/Delete: visible only in accessible channels
/// - TypingStart: visible only in accessible channels
/// - ChannelCreate/Update/Delete/RoleCreate/Update/Delete/MemberUpdate/ServerUpdate: server members only
/// - PresenceUpdate/MemberJoin/MemberLeave/ChannelAck: broadcast to all connected users
///
/// # Arguments
/// * `event` - The event JSON object with "type" and "data" fields
/// * `accessible_channels` - Set of channel IDs this user can access
/// * `member_server_ids` - Set of server IDs this user is a member of
///
/// # Returns
/// true if the event should be sent to this client, false otherwise
pub fn should_dispatch(
    event: &serde_json::Value,
    accessible_channels: &HashSet<i64>,
    member_server_ids: &HashSet<i64>,
) -> bool {
    let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match event_type {
        "MessageCreate" | "MessageUpdate" => {
            let channel_id = event
                .get("data")
                .and_then(|d| d.get("message"))
                .and_then(|m| m.get("channel_id"))
                .and_then(|c| c.as_str())
                .and_then(|c| c.parse::<i64>().ok());
            channel_id.map(|id| accessible_channels.contains(&id)).unwrap_or(false)
        }
        "MessageDelete" | "TypingStart" | "ReactionAdd" | "ReactionRemove" => {
            let channel_id = event
                .get("data")
                .and_then(|d| d.get("channel_id"))
                .and_then(|c| c.as_str())
                .and_then(|c| c.parse::<i64>().ok());
            channel_id.map(|id| accessible_channels.contains(&id)).unwrap_or(false)
        }
        // Server-scoped events: dispatch only to members of the server
        "ChannelCreate" | "ChannelUpdate" => {
            let server_id = event
                .get("data")
                .and_then(|d| d.get("channel"))
                .and_then(|c| c.get("server_id"))
                .and_then(|s| s.as_str())
                .and_then(|s| s.parse::<i64>().ok());
            server_id.map(|id| member_server_ids.contains(&id)).unwrap_or(false)
        }
        "ChannelDelete" | "RoleCreate" | "RoleUpdate" | "RoleDelete" | "MemberUpdate" => {
            let server_id = event
                .get("data")
                .and_then(|d| d.get("server_id"))
                .and_then(|s| s.as_str())
                .and_then(|s| s.parse::<i64>().ok());
            server_id.map(|id| member_server_ids.contains(&id)).unwrap_or(false)
        }
        "ServerUpdate" => {
            let server_id = event
                .get("data")
                .and_then(|d| d.get("server"))
                .and_then(|s| s.get("id"))
                .and_then(|s| s.as_str())
                .and_then(|s| s.parse::<i64>().ok());
            server_id.map(|id| member_server_ids.contains(&id)).unwrap_or(false)
        }
        "ChannelAck" | "PresenceUpdate" | "MemberJoin" | "MemberLeave" => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_sets() -> (HashSet<i64>, HashSet<i64>) {
        (HashSet::new(), HashSet::new())
    }

    #[test]
    fn test_should_dispatch_message_create() {
        let mut channels = HashSet::new();
        channels.insert(12345_i64);
        let servers = HashSet::new();

        let event = serde_json::json!({
            "type": "MessageCreate",
            "data": { "message": { "channel_id": "12345", "content": "hello" } }
        });
        assert!(should_dispatch(&event, &channels, &servers));

        let event_other = serde_json::json!({
            "type": "MessageCreate",
            "data": { "message": { "channel_id": "99999", "content": "hello" } }
        });
        assert!(!should_dispatch(&event_other, &channels, &servers));
    }

    #[test]
    fn test_should_dispatch_typing_start() {
        let mut channels = HashSet::new();
        channels.insert(12345_i64);
        let servers = HashSet::new();

        let event = serde_json::json!({
            "type": "TypingStart",
            "data": { "channel_id": "12345", "user_id": "999" }
        });
        assert!(should_dispatch(&event, &channels, &servers));

        let event_other = serde_json::json!({
            "type": "TypingStart",
            "data": { "channel_id": "99999", "user_id": "999" }
        });
        assert!(!should_dispatch(&event_other, &channels, &servers));
    }

    #[test]
    fn test_should_dispatch_presence_update() {
        let (channels, servers) = empty_sets();
        let event = serde_json::json!({
            "type": "PresenceUpdate",
            "data": { "user_id": "999", "status": "online" }
        });
        assert!(should_dispatch(&event, &channels, &servers));
    }

    #[test]
    fn test_should_dispatch_channel_create() {
        let channels = HashSet::new();
        let mut servers = HashSet::new();
        servers.insert(42_i64);

        let event = serde_json::json!({
            "type": "ChannelCreate",
            "data": { "channel": { "server_id": "42", "id": "999", "name": "new-channel" } }
        });
        assert!(should_dispatch(&event, &channels, &servers));

        let event_other = serde_json::json!({
            "type": "ChannelCreate",
            "data": { "channel": { "server_id": "99", "id": "998", "name": "other" } }
        });
        assert!(!should_dispatch(&event_other, &channels, &servers));
    }

    #[test]
    fn test_should_dispatch_role_create() {
        let channels = HashSet::new();
        let mut servers = HashSet::new();
        servers.insert(42_i64);

        let event = serde_json::json!({
            "type": "RoleCreate",
            "data": { "server_id": "42", "role": { "id": "1", "name": "Admin" } }
        });
        assert!(should_dispatch(&event, &channels, &servers));

        let event_other = serde_json::json!({
            "type": "RoleCreate",
            "data": { "server_id": "99", "role": { "id": "2", "name": "Mod" } }
        });
        assert!(!should_dispatch(&event_other, &channels, &servers));
    }

    #[test]
    fn test_should_dispatch_server_update() {
        let channels = HashSet::new();
        let mut servers = HashSet::new();
        servers.insert(42_i64);

        let event = serde_json::json!({
            "type": "ServerUpdate",
            "data": { "server": { "id": "42", "name": "Updated Server" } }
        });
        assert!(should_dispatch(&event, &channels, &servers));

        let event_other = serde_json::json!({
            "type": "ServerUpdate",
            "data": { "server": { "id": "99", "name": "Other" } }
        });
        assert!(!should_dispatch(&event_other, &channels, &servers));
    }

    #[test]
    fn test_should_dispatch_unknown_type() {
        let (channels, servers) = empty_sets();
        let event = serde_json::json!({"type": "Unknown", "data": {}});
        assert!(!should_dispatch(&event, &channels, &servers));
    }
}
