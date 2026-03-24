//! # Event Route Types
//! Request and response types for event endpoints.
//!
//! ## Depends On
//! - serde (serialization)
//! - chrono (timestamps)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event response body.
#[derive(Debug, Serialize, Clone)]
pub struct EventResponse {
    /// Snowflake event ID
    pub id: String,
    /// Snowflake server ID
    pub server_id: String,
    /// Optional Snowflake channel ID
    pub channel_id: Option<String>,
    /// Snowflake creator user ID
    pub creator_id: String,
    /// Creator's username
    pub creator_username: String,
    /// Event title
    pub title: String,
    /// Optional event description
    pub description: Option<String>,
    /// Location type: 'voice', 'external', or 'stage'
    pub location_type: String,
    /// Optional specific location name
    pub location_name: Option<String>,
    /// Event start timestamp
    pub starts_at: DateTime<Utc>,
    /// Optional event end timestamp
    pub ends_at: Option<DateTime<Utc>>,
    /// Event status
    pub status: String,
    /// Optional cover image URL
    pub cover_image_url: Option<String>,
    /// Number of users RSVP'd
    pub rsvp_count: i64,
    /// Event creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Request body for creating an event.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CreateEventRequest {
    /// Event title (1-100 chars)
    pub title: String,
    /// Optional event description
    pub description: Option<String>,
    /// Location type: 'voice', 'external', or 'stage' (defaults to 'external')
    pub location_type: Option<String>,
    /// Optional specific location name
    pub location_name: Option<String>,
    /// Optional channel ID
    pub channel_id: Option<String>,
    /// Event start timestamp (must be in the future)
    pub starts_at: DateTime<Utc>,
    /// Optional event end timestamp
    pub ends_at: Option<DateTime<Utc>>,
}

/// Request body for updating event status.
#[derive(Debug, Deserialize)]
pub struct UpdateEventRequest {
    /// New status: 'active', 'completed', or 'cancelled'
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_event_request_minimal() {
        let json = r#"{"title":"Game Night","starts_at":"2026-04-01T19:00:00Z"}"#;
        let req: CreateEventRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.title, "Game Night");
        assert_eq!(req.location_type, None);
        assert_eq!(req.description, None);
    }

    #[test]
    fn test_create_event_request_full() {
        let json = r#"{"title":"Movie Night","description":"Watch Inception","location_type":"external","location_name":"Discord","starts_at":"2026-04-01T20:00:00Z","ends_at":"2026-04-01T22:30:00Z"}"#;
        let req: CreateEventRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.title, "Movie Night");
        assert_eq!(req.description, Some("Watch Inception".to_string()));
        assert_eq!(req.location_type, Some("external".to_string()));
    }

    #[test]
    fn test_update_event_request() {
        let json = r#"{"status":"completed"}"#;
        let req: UpdateEventRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.status, "completed");
    }

    #[test]
    fn test_event_response_serialization() {
        let now = Utc::now();
        let response = EventResponse {
            id: "999111".to_string(),
            server_id: "111222".to_string(),
            channel_id: None,
            creator_id: "333444".to_string(),
            creator_username: "alice".to_string(),
            title: "Game Night".to_string(),
            description: Some("Mario Kart tournament".to_string()),
            location_type: "voice".to_string(),
            location_name: Some("Gaming VC".to_string()),
            starts_at: now,
            ends_at: None,
            status: "scheduled".to_string(),
            cover_image_url: None,
            rsvp_count: 5,
            created_at: now,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Game Night"));
        assert!(json.contains("alice"));
        assert!(json.contains("voice"));
    }
}
