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
//! - ChannelAck: dispatched to all connected users (client filters by user_id)
//! - PresenceUpdate: dispatched to all connected users

use std::collections::HashSet;

/// Check if an event should be dispatched to a WebSocket session.
///
/// Different event types have different access requirements:
/// - MessageCreate/Update/Delete: visible only in accessible channels
/// - TypingStart: visible only in accessible channels
/// - PresenceUpdate: visible to all authenticated users (always true)
///
/// # Arguments
/// * `event` - The event JSON object with "type" and "data" fields
/// * `accessible_channels` - Set of channel IDs this user can access
///
/// # Returns
/// true if the event should be sent to this client, false otherwise
pub fn should_dispatch(event: &serde_json::Value, accessible_channels: &HashSet<i64>) -> bool {
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
        "ChannelAck" | "PresenceUpdate" => true,  // broadcast to all connected users
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_should_dispatch_typing_start() {
        let mut channels = HashSet::new();
        channels.insert(12345_i64);

        let event = serde_json::json!({
            "type": "TypingStart",
            "data": { "channel_id": "12345", "user_id": "999" }
        });
        assert!(should_dispatch(&event, &channels));

        let event_other = serde_json::json!({
            "type": "TypingStart",
            "data": { "channel_id": "99999", "user_id": "999" }
        });
        assert!(!should_dispatch(&event_other, &channels));
    }

    #[test]
    fn test_should_dispatch_presence_update() {
        let channels = HashSet::new();
        let event = serde_json::json!({
            "type": "PresenceUpdate",
            "data": { "user_id": "999", "status": "online" }
        });
        assert!(should_dispatch(&event, &channels));
    }

    #[test]
    fn test_should_dispatch_unknown_type() {
        let channels = HashSet::new();
        let event = serde_json::json!({"type": "Unknown", "data": {}});
        assert!(!should_dispatch(&event, &channels));
    }
}
