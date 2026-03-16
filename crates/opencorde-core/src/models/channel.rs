//! # Model: Channel
//! Channels: text, voice, and category.

use crate::snowflake::Snowflake;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Channel type: text, voice, or category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelType {
    /// Text channel for messaging
    Text,
    /// Voice channel for audio
    Voice,
    /// Category to organize channels
    Category,
}

/// Channel: text, voice, or organizational category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    /// Unique channel ID
    pub id: Snowflake,
    /// Server this channel belongs to
    pub server_id: Snowflake,
    /// Channel name
    pub name: String,
    /// Type of channel
    pub channel_type: ChannelType,
    /// Optional topic/description
    pub topic: Option<String>,
    /// Display position in channel list
    pub position: i32,
    /// Optional parent category ID
    pub parent_id: Option<Snowflake>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_creation() {
        let channel = Channel {
            id: Snowflake::new(500),
            server_id: Snowflake::new(100),
            name: "general".to_string(),
            channel_type: ChannelType::Text,
            topic: Some("General discussion".to_string()),
            position: 0,
            parent_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(channel.name, "general");
        assert_eq!(channel.channel_type, ChannelType::Text);
    }

    #[test]
    fn test_voice_channel() {
        let channel = Channel {
            id: Snowflake::new(501),
            server_id: Snowflake::new(100),
            name: "General Voice".to_string(),
            channel_type: ChannelType::Voice,
            topic: None,
            position: 1,
            parent_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(channel.channel_type, ChannelType::Voice);
    }

    #[test]
    fn test_category_channel() {
        let channel = Channel {
            id: Snowflake::new(502),
            server_id: Snowflake::new(100),
            name: "Announcements".to_string(),
            channel_type: ChannelType::Category,
            topic: None,
            position: 2,
            parent_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(channel.channel_type, ChannelType::Category);
    }

    #[test]
    fn test_channel_with_parent() {
        let channel = Channel {
            id: Snowflake::new(503),
            server_id: Snowflake::new(100),
            name: "sub-general".to_string(),
            channel_type: ChannelType::Text,
            topic: None,
            position: 0,
            parent_id: Some(Snowflake::new(502)),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(channel.parent_id.is_some());
        assert_eq!(channel.parent_id.unwrap(), Snowflake::new(502));
    }

    #[test]
    fn test_channel_serialization() {
        let channel = Channel {
            id: Snowflake::new(504),
            server_id: Snowflake::new(100),
            name: "test-channel".to_string(),
            channel_type: ChannelType::Text,
            topic: Some("Test topic".to_string()),
            position: 5,
            parent_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&channel).unwrap();
        let deserialized: Channel = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, channel.id);
        assert_eq!(deserialized.name, channel.name);
        assert_eq!(deserialized.channel_type, channel.channel_type);
    }

    #[test]
    fn test_channel_type_serialization() {
        let types = vec![ChannelType::Text, ChannelType::Voice, ChannelType::Category];

        for ty in types {
            let json = serde_json::to_string(&ty).unwrap();
            let deserialized: ChannelType = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, ty);
        }
    }
}
