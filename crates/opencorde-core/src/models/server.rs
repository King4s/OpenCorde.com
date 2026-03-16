//! # Model: Server
//! Server (guild/community) representation.

use crate::snowflake::Snowflake;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Server: a community space with channels, roles, and members.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    /// Unique server ID
    pub id: Snowflake,
    /// Server name
    pub name: String,
    /// ID of the owner user
    pub owner_id: Snowflake,
    /// Optional server icon URL
    pub icon_url: Option<String>,
    /// Optional server description
    pub description: Option<String>,
    /// Server creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = Server {
            id: Snowflake::new(100),
            name: "My Server".to_string(),
            owner_id: Snowflake::new(200),
            icon_url: None,
            description: Some("A test server".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(server.name, "My Server");
        assert_eq!(server.owner_id, Snowflake::new(200));
    }

    #[test]
    fn test_server_serialization() {
        let server = Server {
            id: Snowflake::new(300),
            name: "Test Server".to_string(),
            owner_id: Snowflake::new(400),
            icon_url: Some("https://example.com/icon.png".to_string()),
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&server).unwrap();
        let deserialized: Server = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, server.id);
        assert_eq!(deserialized.name, server.name);
        assert_eq!(deserialized.owner_id, server.owner_id);
    }

    #[test]
    fn test_server_with_all_fields() {
        let now = Utc::now();
        let server = Server {
            id: Snowflake::new(999),
            name: "Full Server".to_string(),
            owner_id: Snowflake::new(888),
            icon_url: Some("https://example.com/full_icon.png".to_string()),
            description: Some("Full description".to_string()),
            created_at: now,
            updated_at: now,
        };

        assert_eq!(server.description.as_ref().unwrap(), "Full description");
        assert!(server.icon_url.is_some());
    }
}
