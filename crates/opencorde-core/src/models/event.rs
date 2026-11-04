//! # Model: Event
//! Scheduled server event representation for gateway serialization.

use crate::snowflake::Snowflake;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event: a scheduled gathering in a server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event ID
    pub id: Snowflake,
    /// Parent server ID
    pub server_id: Snowflake,
    /// Optional associated channel ID
    pub channel_id: Option<Snowflake>,
    /// ID of the user who created the event
    pub creator_id: Snowflake,
    /// Event title
    pub title: String,
    /// Optional description
    pub description: Option<String>,
    /// Location type: 'voice', 'external', or 'stage'
    pub location_type: String,
    /// Optional specific location name
    pub location_name: Option<String>,
    /// Event start timestamp
    pub starts_at: DateTime<Utc>,
    /// Optional event end timestamp
    pub ends_at: Option<DateTime<Utc>>,
    /// Current status: 'scheduled', 'active', 'completed', 'cancelled'
    pub status: String,
    /// Optional cover image URL
    pub cover_image_url: Option<String>,
    /// Event creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Number of users who RSVP'd
    pub rsvp_count: i64,
    /// Username of the creator
    pub creator_username: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event {
            id: Snowflake::new(1000),
            server_id: Snowflake::new(100),
            channel_id: Some(Snowflake::new(500)),
            creator_id: Snowflake::new(200),
            title: "Game Night".to_string(),
            description: Some("Weekly game night".to_string()),
            location_type: "voice".to_string(),
            location_name: None,
            starts_at: Utc::now(),
            ends_at: None,
            status: "scheduled".to_string(),
            cover_image_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            rsvp_count: 5,
            creator_username: "alice".to_string(),
        };

        assert_eq!(event.title, "Game Night");
        assert_eq!(event.status, "scheduled");
    }

    #[test]
    fn test_event_serialization() {
        let event = Event {
            id: Snowflake::new(1000),
            server_id: Snowflake::new(100),
            channel_id: None,
            creator_id: Snowflake::new(200),
            title: "Test Event".to_string(),
            description: None,
            location_type: "external".to_string(),
            location_name: Some("Central Park".to_string()),
            starts_at: Utc::now(),
            ends_at: None,
            status: "active".to_string(),
            cover_image_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            rsvp_count: 0,
            creator_username: "bob".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, event.id);
        assert_eq!(deserialized.title, "Test Event");
    }
}
