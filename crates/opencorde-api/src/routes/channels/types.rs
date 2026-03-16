//! # Channel Route Types
//! Request and response types for channel endpoints.
//!
//! ## Depends On
//! - serde (serialization)
//! - chrono (timestamps)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Channel response body.
#[derive(Debug, Serialize, Clone)]
pub struct ChannelResponse {
    /// Snowflake channel ID
    pub id: String,
    /// Snowflake server ID
    pub server_id: String,
    /// Channel name (1-100 chars)
    pub name: String,
    /// Channel type (0=Text, 1=Voice, 2=Category)
    pub channel_type: i16,
    /// Channel topic (optional)
    pub topic: Option<String>,
    /// Display position in channel list
    pub position: i32,
    /// Parent category ID (optional, for nested channels)
    pub parent_id: Option<String>,
    /// Channel creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Request body for creating a channel.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CreateChannelRequest {
    /// Channel name (must be 1-100 chars)
    pub name: String,
    /// Channel type: 0=Text, 1=Voice, 2=Category (optional, defaults to 0)
    pub channel_type: Option<i16>,
    /// Channel topic (optional)
    pub topic: Option<String>,
    /// Parent category ID (optional, for nested channels)
    pub parent_id: Option<String>,
}

/// Request body for updating a channel.
#[derive(Debug, Deserialize)]
pub struct UpdateChannelRequest {
    /// Optional new channel name
    pub name: Option<String>,
    /// Optional new channel topic
    pub topic: Option<String>,
    /// Optional new position
    pub position: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_channel_request_deserialization() {
        let json = r#"{"name":"general","channel_type":0,"topic":"Main chat"}"#;
        let req: CreateChannelRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "general");
        assert_eq!(req.channel_type, Some(0));
        assert_eq!(req.topic, Some("Main chat".to_string()));
    }

    #[test]
    fn test_create_channel_request_minimal() {
        let json = r#"{"name":"announcements"}"#;
        let req: CreateChannelRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "announcements");
        assert_eq!(req.channel_type, None);
        assert_eq!(req.topic, None);
    }

    #[test]
    fn test_update_channel_request_deserialization() {
        let json = r#"{"name":"updated","topic":"New topic","position":5}"#;
        let req: UpdateChannelRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, Some("updated".to_string()));
        assert_eq!(req.topic, Some("New topic".to_string()));
        assert_eq!(req.position, Some(5));
    }

    #[test]
    fn test_channel_response_serialization() {
        let now = Utc::now();
        let response = ChannelResponse {
            id: "555666".to_string(),
            server_id: "111222".to_string(),
            name: "general".to_string(),
            channel_type: 0,
            topic: Some("Main discussion".to_string()),
            position: 0,
            parent_id: None,
            created_at: now,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("general"));
        assert!(json.contains("555666"));
        assert!(json.contains("111222"));
    }

    #[test]
    fn test_channel_response_with_parent() {
        let now = Utc::now();
        let response = ChannelResponse {
            id: "777888".to_string(),
            server_id: "111222".to_string(),
            name: "category-topic".to_string(),
            channel_type: 2,
            topic: None,
            position: 1,
            parent_id: Some("666777".to_string()),
            created_at: now,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("666777"));
    }
}
