//! # Events
//! Re-export gateway events and core serialization tests.

pub use crate::gateway::GatewayEvent;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Channel, ChannelType, Message, Server, UserStatus, VoiceState};
    use crate::snowflake::Snowflake;
    use chrono::Utc;

    #[test]
    fn test_hello_event() {
        let event = GatewayEvent::Hello {
            heartbeat_interval: 45000,
        };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: GatewayEvent = serde_json::from_str(&json).unwrap();
        match deserialized {
            GatewayEvent::Hello { heartbeat_interval } => {
                assert_eq!(heartbeat_interval, 45000);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_identify_event() {
        let event = GatewayEvent::Identify {
            token: "token_123".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"Identify\""));
        let deserialized: GatewayEvent = serde_json::from_str(&json).unwrap();
        match deserialized {
            GatewayEvent::Identify { token } => {
                assert_eq!(token, "token_123");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_message_events() {
        let message = Message {
            id: Snowflake::new(1000),
            channel_id: Snowflake::new(500),
            author_id: Snowflake::new(200),
            content: "Hello!".to_string(),
            attachments: vec![],
            edited_at: None,
            created_at: Utc::now(),
        };

        let event = GatewayEvent::MessageCreate {
            message: message.clone(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: GatewayEvent = serde_json::from_str(&json).unwrap();
        match deserialized {
            GatewayEvent::MessageCreate { message: msg } => {
                assert_eq!(msg.content, "Hello!");
            }
            _ => panic!("Wrong event type"),
        }

        let delete_event = GatewayEvent::MessageDelete {
            channel_id: Snowflake::new(500),
            message_id: Snowflake::new(1000),
        };
        let json = serde_json::to_string(&delete_event).unwrap();
        let deserialized: GatewayEvent = serde_json::from_str(&json).unwrap();
        match deserialized {
            GatewayEvent::MessageDelete {
                channel_id,
                message_id,
            } => {
                assert_eq!(channel_id, Snowflake::new(500));
                assert_eq!(message_id, Snowflake::new(1000));
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_typing_and_presence_events() {
        let now = Utc::now();
        let typing_event = GatewayEvent::TypingStart {
            channel_id: Snowflake::new(500),
            user_id: Snowflake::new(200),
            timestamp: now,
        };
        let json = serde_json::to_string(&typing_event).unwrap();
        let _: GatewayEvent = serde_json::from_str(&json).unwrap();

        let presence_event = GatewayEvent::PresenceUpdate {
            user_id: Snowflake::new(200),
            status: UserStatus::Idle,
        };
        let json = serde_json::to_string(&presence_event).unwrap();
        let deserialized: GatewayEvent = serde_json::from_str(&json).unwrap();
        match deserialized {
            GatewayEvent::PresenceUpdate { status, .. } => {
                assert_eq!(status, UserStatus::Idle);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_voice_state_update_event() {
        let voice_state = VoiceState {
            user_id: Snowflake::new(200),
            channel_id: Snowflake::new(800),
            session_id: "abc123".to_string(),
            self_mute: false,
            self_deaf: false,
            joined_at: Utc::now(),
        };

        let event = GatewayEvent::VoiceStateUpdate {
            voice_state: voice_state.clone(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: GatewayEvent = serde_json::from_str(&json).unwrap();

        match deserialized {
            GatewayEvent::VoiceStateUpdate { voice_state: vs } => {
                assert_eq!(vs.user_id, voice_state.user_id);
                assert_eq!(vs.channel_id, voice_state.channel_id);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_server_events() {
        let server = Server {
            id: Snowflake::new(100),
            name: "New Server".to_string(),
            owner_id: Snowflake::new(200),
            icon_url: None,
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let create_event = GatewayEvent::ServerCreate {
            server: server.clone(),
        };
        let json = serde_json::to_string(&create_event).unwrap();
        let deserialized: GatewayEvent = serde_json::from_str(&json).unwrap();
        match deserialized {
            GatewayEvent::ServerCreate { server: s } => {
                assert_eq!(s.name, "New Server");
            }
            _ => panic!("Wrong event type"),
        }

        let delete_event = GatewayEvent::ServerDelete {
            server_id: Snowflake::new(100),
        };
        let json = serde_json::to_string(&delete_event).unwrap();
        assert!(json.contains("\"type\":\"ServerDelete\""));
    }

    #[test]
    fn test_channel_events() {
        let channel = Channel {
            id: Snowflake::new(500),
            server_id: Snowflake::new(100),
            name: "general".to_string(),
            channel_type: ChannelType::Text,
            topic: None,
            position: 0,
            parent_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let create_event = GatewayEvent::ChannelCreate {
            channel: channel.clone(),
        };
        let json = serde_json::to_string(&create_event).unwrap();
        let deserialized: GatewayEvent = serde_json::from_str(&json).unwrap();
        match deserialized {
            GatewayEvent::ChannelCreate { channel: ch } => {
                assert_eq!(ch.name, "general");
            }
            _ => panic!("Wrong event type"),
        }

        let delete_event = GatewayEvent::ChannelDelete {
            channel_id: Snowflake::new(500),
        };
        let json = serde_json::to_string(&delete_event).unwrap();
        assert!(json.contains("\"type\":\"ChannelDelete\""));
    }

    #[test]
    fn test_member_events() {
        let leave_event = GatewayEvent::MemberLeave {
            server_id: Snowflake::new(100),
            user_id: Snowflake::new(200),
        };

        let json = serde_json::to_string(&leave_event).unwrap();
        let deserialized: GatewayEvent = serde_json::from_str(&json).unwrap();

        match deserialized {
            GatewayEvent::MemberLeave { server_id, user_id } => {
                assert_eq!(server_id, Snowflake::new(100));
                assert_eq!(user_id, Snowflake::new(200));
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_heartbeat_events() {
        let heartbeat = GatewayEvent::Heartbeat;
        let json = serde_json::to_string(&heartbeat).unwrap();
        assert!(json.contains("\"type\":\"Heartbeat\""));

        let ack = GatewayEvent::HeartbeatAck;
        let json = serde_json::to_string(&ack).unwrap();
        assert!(json.contains("\"type\":\"HeartbeatAck\""));
    }
}
