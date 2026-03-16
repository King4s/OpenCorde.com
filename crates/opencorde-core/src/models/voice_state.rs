//! # Model: VoiceState
//! User voice connection state in a channel.

use crate::snowflake::Snowflake;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Voice connection state: where and how a user is connected to voice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceState {
    /// User ID
    pub user_id: Snowflake,
    /// Voice channel ID
    pub channel_id: Snowflake,
    /// Voice session ID (unique per connection)
    pub session_id: String,
    /// Whether the user muted themselves
    pub self_mute: bool,
    /// Whether the user deafened themselves
    pub self_deaf: bool,
    /// When the user joined the voice channel
    pub joined_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_state_creation() {
        let state = VoiceState {
            user_id: Snowflake::new(200),
            channel_id: Snowflake::new(800),
            session_id: "abc123def456".to_string(),
            self_mute: false,
            self_deaf: false,
            joined_at: Utc::now(),
        };

        assert_eq!(state.user_id, Snowflake::new(200));
        assert_eq!(state.channel_id, Snowflake::new(800));
        assert!(!state.self_mute);
        assert!(!state.self_deaf);
    }

    #[test]
    fn test_voice_state_muted() {
        let state = VoiceState {
            user_id: Snowflake::new(201),
            channel_id: Snowflake::new(800),
            session_id: "xyz789".to_string(),
            self_mute: true,
            self_deaf: false,
            joined_at: Utc::now(),
        };

        assert!(state.self_mute);
        assert!(!state.self_deaf);
    }

    #[test]
    fn test_voice_state_deafened() {
        let state = VoiceState {
            user_id: Snowflake::new(202),
            channel_id: Snowflake::new(800),
            session_id: "session999".to_string(),
            self_mute: false,
            self_deaf: true,
            joined_at: Utc::now(),
        };

        assert!(!state.self_mute);
        assert!(state.self_deaf);
    }

    #[test]
    fn test_voice_state_muted_and_deafened() {
        let state = VoiceState {
            user_id: Snowflake::new(203),
            channel_id: Snowflake::new(800),
            session_id: "muted_deaf".to_string(),
            self_mute: true,
            self_deaf: true,
            joined_at: Utc::now(),
        };

        assert!(state.self_mute);
        assert!(state.self_deaf);
    }

    #[test]
    fn test_voice_state_serialization() {
        let state = VoiceState {
            user_id: Snowflake::new(204),
            channel_id: Snowflake::new(800),
            session_id: "serialize_test".to_string(),
            self_mute: true,
            self_deaf: false,
            joined_at: Utc::now(),
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: VoiceState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.user_id, state.user_id);
        assert_eq!(deserialized.channel_id, state.channel_id);
        assert_eq!(deserialized.session_id, state.session_id);
        assert_eq!(deserialized.self_mute, state.self_mute);
    }

    #[test]
    fn test_voice_state_session_id_uniqueness() {
        let state1 = VoiceState {
            user_id: Snowflake::new(205),
            channel_id: Snowflake::new(800),
            session_id: "session_001".to_string(),
            self_mute: false,
            self_deaf: false,
            joined_at: Utc::now(),
        };

        let state2 = VoiceState {
            user_id: Snowflake::new(206),
            channel_id: Snowflake::new(800),
            session_id: "session_002".to_string(),
            self_mute: false,
            self_deaf: false,
            joined_at: Utc::now(),
        };

        assert_ne!(state1.session_id, state2.session_id);
    }
}
