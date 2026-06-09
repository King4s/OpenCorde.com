//! # Model: Application
//! Registered OAuth2 application (bot/client).
//!
//! Applications are created by users and can be installed to servers.
//! An application may have an associated bot user for automated interactions.

use crate::snowflake::Snowflake;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Application: an OAuth2 application registered by a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    /// Unique application ID (Snowflake)
    pub id: Snowflake,
    /// Application name (max 32 chars)
    pub name: String,
    /// Short description
    pub description: Option<String>,
    /// Icon URL
    pub icon_url: Option<String>,
    /// Owner user ID
    pub owner_user_id: Snowflake,
    /// Bitfield flags (reserved for future use)
    pub flags: i64,
    /// Whether the application is publicly discoverable
    pub is_public: bool,
    /// Allowed OAuth2 redirect URIs
    pub redirect_uris: Vec<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_application_creation() {
        let app = Application {
            id: Snowflake::new(1000),
            name: "MyBot".to_string(),
            description: Some("A test bot".to_string()),
            icon_url: None,
            owner_user_id: Snowflake::new(200),
            flags: 0,
            is_public: true,
            redirect_uris: vec!["https://example.com/callback".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(app.name, "MyBot");
        assert_eq!(app.owner_user_id, Snowflake::new(200));
        assert!(app.is_public);
        assert_eq!(app.redirect_uris.len(), 1);
    }

    #[test]
    fn test_application_serialization() {
        let app = Application {
            id: Snowflake::new(2000),
            name: "TestApp".to_string(),
            description: None,
            icon_url: Some("https://example.com/icon.png".to_string()),
            owner_user_id: Snowflake::new(300),
            flags: 0,
            is_public: false,
            redirect_uris: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&app).unwrap();
        let deserialized: Application = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, app.id);
        assert_eq!(deserialized.name, app.name);
        assert_eq!(deserialized.is_public, false);
        assert!(deserialized.redirect_uris.is_empty());
    }

    #[test]
    fn test_application_private_default() {
        let app = Application {
            id: Snowflake::new(3000),
            name: "PrivateBot".to_string(),
            description: None,
            icon_url: None,
            owner_user_id: Snowflake::new(400),
            flags: 0,
            is_public: false,
            redirect_uris: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(!app.is_public);
    }

    #[test]
    fn test_application_multiple_redirect_uris() {
        let uris = vec![
            "https://app.example.com/oauth/callback".to_string(),
            "https://app.example.com/oauth/callback2".to_string(),
        ];
        let app = Application {
            id: Snowflake::new(4000),
            name: "MultiURI".to_string(),
            description: None,
            icon_url: None,
            owner_user_id: Snowflake::new(500),
            flags: 0,
            is_public: true,
            redirect_uris: uris.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(app.redirect_uris.len(), 2);
        assert_eq!(app.redirect_uris, uris);
    }
}
