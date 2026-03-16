//! # Voice Route Types
//! Request/response types for voice endpoints.
//!
//! Provides serializable/deserializable types for voice channel
//! management and LiveKit token requests.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Request body for joining a voice channel.
#[derive(Debug, Deserialize)]
pub struct JoinVoiceRequest {
    /// Snowflake channel ID (as string)
    pub channel_id: String,
}

/// Request body for updating voice state (mute/deafen).
#[derive(Debug, Deserialize)]
pub struct UpdateVoiceStateRequest {
    /// Whether user has muted their microphone
    pub self_mute: bool,
    /// Whether user has deafened themselves
    pub self_deaf: bool,
}

/// Request body for obtaining a LiveKit token.
#[derive(Debug, Deserialize)]
pub struct LiveKitTokenRequest {
    /// Snowflake channel ID (as string)
    pub channel_id: String,
}

/// Voice state response (no token).
#[derive(Debug, Serialize)]
pub struct VoiceStateResponse {
    /// Snowflake user ID
    pub user_id: String,
    /// Snowflake channel ID
    pub channel_id: String,
    /// Session identifier
    pub session_id: String,
    /// Whether user has muted their microphone
    pub self_mute: bool,
    /// Whether user has deafened themselves
    pub self_deaf: bool,
    /// When user joined the voice channel (ISO 8601)
    pub joined_at: DateTime<Utc>,
}

/// Join voice response (includes LiveKit token).
#[derive(Debug, Serialize)]
pub struct JoinVoiceResponse {
    /// Voice state information
    pub voice_state: VoiceStateResponse,
    /// LiveKit WebRTC access token
    pub livekit_token: String,
}

/// LiveKit token response (token + URL).
#[derive(Debug, Serialize)]
pub struct LiveKitTokenResponse {
    /// LiveKit WebRTC access token
    pub token: String,
    /// LiveKit WebSocket URL for connection
    pub url: String,
}

/// Participant in a voice channel.
#[derive(Debug, Serialize)]
pub struct VoiceParticipant {
    /// Snowflake user ID
    pub user_id: String,
    /// Session identifier
    pub session_id: String,
    /// Whether user has muted their microphone
    pub self_mute: bool,
    /// Whether user has deafened themselves
    pub self_deaf: bool,
    /// When user joined the voice channel (ISO 8601)
    pub joined_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_voice_request_deserialization() {
        let json = r#"{"channel_id": "123456789"}"#;
        let req: JoinVoiceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.channel_id, "123456789");
    }

    #[test]
    fn test_update_voice_state_request() {
        let json = r#"{"self_mute": true, "self_deaf": false}"#;
        let req: UpdateVoiceStateRequest = serde_json::from_str(json).unwrap();
        assert!(req.self_mute);
        assert!(!req.self_deaf);
    }

    #[test]
    fn test_livekit_token_request() {
        let json = r#"{"channel_id": "987654321"}"#;
        let req: LiveKitTokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.channel_id, "987654321");
    }

    #[test]
    fn test_voice_state_response_serialization() {
        let response = VoiceStateResponse {
            user_id: "111".to_string(),
            channel_id: "222".to_string(),
            session_id: "session-abc".to_string(),
            self_mute: false,
            self_deaf: false,
            joined_at: Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("111"));
        assert!(json.contains("222"));
        assert!(json.contains("session-abc"));
    }

    #[test]
    fn test_livekit_token_response_serialization() {
        let response = LiveKitTokenResponse {
            token: "eyJhbGc...".to_string(),
            url: "wss://livekit.example.com".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("eyJhbGc"));
        assert!(json.contains("wss://livekit.example.com"));
    }
}
