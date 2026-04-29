//! # LiveKit Token Generation
//! JWT token generation for LiveKit WebRTC access.
//!
//! Generates standard JWT tokens with LiveKit-specific claims
//! for WebRTC room access.

use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::Serialize;
use uuid::Uuid;

/// LiveKit video grant permissions.
#[derive(Debug, Serialize)]
pub struct LiveKitVideoGrant {
    /// Room name (channel ID)
    #[serde(rename = "room")]
    pub room: String,

    /// Allow joining the room
    #[serde(rename = "roomJoin")]
    pub room_join: bool,

    /// Allow publishing media
    #[serde(rename = "canPublish")]
    pub can_publish: bool,

    /// Allow subscribing to other participants
    #[serde(rename = "canSubscribe")]
    pub can_subscribe: bool,
}

/// LiveKit JWT claims.
///
/// Standard JWT payload for LiveKit WebRTC access.
/// See: https://docs.livekit.io/reference/server-apis/access-tokens/
#[derive(Debug, Serialize)]
pub struct LiveKitClaims {
    /// Issuer (API key)
    pub iss: String,

    /// Subject (user identity)
    pub sub: String,

    /// Expiration time (Unix timestamp)
    pub exp: i64,

    /// Issued at (Unix timestamp)
    pub iat: i64,

    /// Not before (Unix timestamp)
    pub nbf: i64,

    /// JWT ID (unique token identifier)
    pub jti: String,

    /// Video grant (permissions)
    pub video: LiveKitVideoGrant,
}

/// Generate a LiveKit access token.
///
/// Creates a JWT token for WebRTC room access with the specified permissions.
///
/// # Arguments
/// * `api_key` - LiveKit API key (used as issuer)
/// * `api_secret` - LiveKit API secret (used for signing)
/// * `user_id` - User identity (Snowflake ID as string)
/// * `room_name` - Room name (channel ID as string)
/// * `expiry_seconds` - Token expiry duration in seconds (default: 3600)
/// * `can_publish` - Whether the user may publish media to the room
///
/// # Returns
/// JWT token string or error if signing fails.
///
/// # Errors
/// Returns jsonwebtoken error if token generation fails.
#[tracing::instrument(skip(api_secret))]
pub fn create_livekit_token(
    api_key: &str,
    api_secret: &str,
    user_id: &str,
    room_name: &str,
    expiry_seconds: u64,
    can_publish: bool,
) -> Result<String, jsonwebtoken::errors::Error> {
    tracing::info!(
        api_key_hint = api_key.chars().take(6).collect::<String>(),
        user_id = %user_id,
        room_name = %room_name,
        expiry_seconds = expiry_seconds,
        can_publish = can_publish,
        "generating LiveKit token"
    );

    let now = Utc::now().timestamp();
    let exp = now + expiry_seconds as i64;

    let claims = LiveKitClaims {
        iss: api_key.to_string(),
        sub: user_id.to_string(),
        exp,
        iat: now,
        nbf: now,
        jti: Uuid::new_v4().to_string(),
        video: LiveKitVideoGrant {
            room: room_name.to_string(),
            room_join: true,
            can_publish,
            can_subscribe: true,
        },
    };

    let key = EncodingKey::from_secret(api_secret.as_bytes());
    let token = encode(&Header::default(), &claims, &key)?;

    tracing::debug!(
        user_id = %user_id,
        room_name = %room_name,
        token_length = token.len(),
        "LiveKit token generated"
    );

    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_livekit_video_grant_serialization() {
        let grant = LiveKitVideoGrant {
            room: "room-123".to_string(),
            room_join: true,
            can_publish: true,
            can_subscribe: true,
        };

        let json = serde_json::to_string(&grant).unwrap();
        assert!(json.contains("room-123"));
        assert!(json.contains("roomJoin"));
        assert!(json.contains("canPublish"));
        assert!(json.contains("canSubscribe"));
    }

    #[test]
    fn test_livekit_claims_serialization() {
        let claims = LiveKitClaims {
            iss: "api-key".to_string(),
            sub: "user-123".to_string(),
            exp: 1234567890,
            iat: 1234567800,
            nbf: 1234567800,
            jti: "uuid-here".to_string(),
            video: LiveKitVideoGrant {
                room: "room-123".to_string(),
                room_join: true,
                can_publish: true,
                can_subscribe: true,
            },
        };

        let json = serde_json::to_string(&claims).unwrap();
        assert!(json.contains("user-123"));
        assert!(json.contains("room-123"));
        assert!(json.contains("api-key"));
    }

    #[test]
    fn test_create_livekit_token_valid() {
        let result = create_livekit_token(
            "test-api-key",
            "test-api-secret",
            "user-123",
            "room-456",
            3600,
            true,
        );

        assert!(result.is_ok());
        let token = result.unwrap();
        assert!(!token.is_empty());
        // JWT tokens are base64-encoded with dots
        assert_eq!(token.matches('.').count(), 2);
    }

    #[test]
    fn test_create_livekit_token_different_expiry() {
        let token1 = create_livekit_token(
            "key", "secret", "user-1", "room-1", 1800, // 30 min
            true,
        )
        .unwrap();

        let token2 = create_livekit_token(
            "key", "secret", "user-1", "room-1", 7200, // 2 hours
            true,
        )
        .unwrap();

        // Tokens should be different due to different exp/iat/jti
        assert_ne!(token1, token2);
    }
}
