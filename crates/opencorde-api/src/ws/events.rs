//! # WebSocket Events
//! Helper functions for creating gateway event JSON payloads.
//!
//! All events follow the format: `{ "type": "EventName", "data": {...} }`
//!
//! ## Depends On
//! - serde_json (JSON serialization)

use serde_json::{Value, json};

/// Create a MESSAGE_CREATE event.
///
/// Fired when a message is posted to a channel.
pub fn message_create(message: &Value) -> String {
    json!({
        "type": "MessageCreate",
        "data": { "message": message }
    })
    .to_string()
}

/// Create a MESSAGE_UPDATE event.
///
/// Fired when a message is edited.
pub fn message_update(message: &Value) -> String {
    json!({
        "type": "MessageUpdate",
        "data": { "message": message }
    })
    .to_string()
}

/// Create a MESSAGE_DELETE event.
///
/// Fired when a message is deleted.
pub fn message_delete(channel_id: &str, message_id: &str) -> String {
    json!({
        "type": "MessageDelete",
        "data": {
            "channel_id": channel_id,
            "message_id": message_id
        }
    })
    .to_string()
}

/// Create a TYPING_START event.
///
/// Fired when a user starts typing in a channel.
pub fn typing_start(channel_id: &str, user_id: &str) -> String {
    json!({
        "type": "TypingStart",
        "data": {
            "channel_id": channel_id,
            "user_id": user_id
        }
    })
    .to_string()
}

/// Create a PRESENCE_UPDATE event.
///
/// Fired when a user's status changes.
pub fn presence_update(user_id: &str, status: &str) -> String {
    json!({
        "type": "PresenceUpdate",
        "data": {
            "user_id": user_id,
            "status": status
        }
    })
    .to_string()
}

/// Create a VOICE_STATE_UPDATE event.
///
/// Fired when a user connects/disconnects from voice or changes channels.
pub fn voice_state_update(user_id: &str, voice_state: &Value) -> String {
    json!({
        "type": "VoiceStateUpdate",
        "data": {
            "user_id": user_id,
            "voice_state": voice_state
        }
    })
    .to_string()
}

/// Create a SERVER_CREATE event.
///
/// Fired when a new server is created.
pub fn server_create(server: &Value) -> String {
    json!({
        "type": "ServerCreate",
        "data": { "server": server }
    })
    .to_string()
}

/// Create a CHANNEL_CREATE event.
///
/// Fired when a new channel is created.
pub fn channel_create(channel: &Value) -> String {
    json!({
        "type": "ChannelCreate",
        "data": { "channel": channel }
    })
    .to_string()
}

/// Create a MEMBER_JOIN event.
///
/// Fired when a user joins a server.
pub fn member_join(server_id: &str, member: &Value) -> String {
    json!({
        "type": "MemberJoin",
        "data": {
            "server_id": server_id,
            "member": member
        }
    })
    .to_string()
}

/// Create a MEMBER_LEAVE event.
///
/// Fired when a user leaves a server.
pub fn member_leave(server_id: &str, user_id: &str) -> String {
    json!({
        "type": "MemberLeave",
        "data": {
            "server_id": server_id,
            "user_id": user_id
        }
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_create() {
        let msg = json!({"id": "123", "content": "hello"});
        let result = message_create(&msg);
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["type"], "MessageCreate");
        assert_eq!(parsed["data"]["message"]["id"], "123");
    }

    #[test]
    fn test_message_delete() {
        let result = message_delete("chan_1", "msg_1");
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["type"], "MessageDelete");
        assert_eq!(parsed["data"]["channel_id"], "chan_1");
        assert_eq!(parsed["data"]["message_id"], "msg_1");
    }

    #[test]
    fn test_typing_start() {
        let result = typing_start("chan_2", "user_2");
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["type"], "TypingStart");
        assert_eq!(parsed["data"]["channel_id"], "chan_2");
        assert_eq!(parsed["data"]["user_id"], "user_2");
    }

    #[test]
    fn test_presence_update() {
        let result = presence_update("user_1", "online");
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["type"], "PresenceUpdate");
        assert_eq!(parsed["data"]["user_id"], "user_1");
        assert_eq!(parsed["data"]["status"], "online");
    }

    #[test]
    fn test_voice_state_update() {
        let voice = json!({"channel_id": "voice_1", "muted": false});
        let result = voice_state_update("user_1", &voice);
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["type"], "VoiceStateUpdate");
        assert_eq!(parsed["data"]["user_id"], "user_1");
        assert_eq!(parsed["data"]["voice_state"]["channel_id"], "voice_1");
    }

    #[test]
    fn test_server_create() {
        let server = json!({"id": "srv_1", "name": "Test Server"});
        let result = server_create(&server);
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["type"], "ServerCreate");
        assert_eq!(parsed["data"]["server"]["name"], "Test Server");
    }

    #[test]
    fn test_channel_create() {
        let channel = json!({"id": "ch_1", "name": "general"});
        let result = channel_create(&channel);
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["type"], "ChannelCreate");
        assert_eq!(parsed["data"]["channel"]["name"], "general");
    }

    #[test]
    fn test_member_join() {
        let member = json!({"user_id": "u_1", "role": "member"});
        let result = member_join("srv_1", &member);
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["type"], "MemberJoin");
        assert_eq!(parsed["data"]["server_id"], "srv_1");
    }

    #[test]
    fn test_member_leave() {
        let result = member_leave("srv_1", "u_1");
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["type"], "MemberLeave");
        assert_eq!(parsed["data"]["server_id"], "srv_1");
        assert_eq!(parsed["data"]["user_id"], "u_1");
    }
}
