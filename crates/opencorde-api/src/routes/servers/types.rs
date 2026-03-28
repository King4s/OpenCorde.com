//! # Server Route Types
//! Request and response types for server endpoints.
//!
//! ## Depends On
//! - serde (serialization)
//! - chrono (timestamps)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Server response body.
#[derive(Debug, Serialize, Clone)]
pub struct ServerResponse {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    pub description: Option<String>,
    pub vanity_url: Option<String>,
    /// 0=NONE 1=LOW 2=MEDIUM 3=HIGH 4=VERY_HIGH
    pub verification_level: i16,
    /// 0=DISABLED 1=MEMBERS_WITHOUT_ROLES 2=ALL_MEMBERS
    pub explicit_content_filter: i16,
    /// 0=ALL_MESSAGES 1=ONLY_MENTIONS
    pub default_notifications: i16,
    pub system_channel_id: Option<String>,
    pub rules_channel_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Request body for creating a server.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CreateServerRequest {
    /// Server name (must be 1-100 chars)
    pub name: String,
    /// Optional server description
    pub description: Option<String>,
}

/// Request body for updating a server.
#[derive(Debug, Deserialize)]
pub struct UpdateServerRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub verification_level: Option<i16>,
    pub explicit_content_filter: Option<i16>,
    pub default_notifications: Option<i16>,
    pub vanity_url: Option<String>,
    pub system_channel_id: Option<String>,
    pub rules_channel_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_server_request_deserialization() {
        let json = r#"{"name":"Test Server","description":"A test"}"#;
        let req: CreateServerRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Test Server");
        assert_eq!(req.description, Some("A test".to_string()));
    }

    #[test]
    fn test_create_server_request_without_description() {
        let json = r#"{"name":"Test Server"}"#;
        let req: CreateServerRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Test Server");
        assert_eq!(req.description, None);
    }

    #[test]
    fn test_update_server_request_deserialization() {
        let json = r#"{"name":"Updated","description":"New desc"}"#;
        let req: UpdateServerRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, Some("Updated".to_string()));
        assert_eq!(req.description, Some("New desc".to_string()));
    }

    #[test]
    fn test_server_response_serialization() {
        let now = Utc::now();
        let response = ServerResponse {
            id: "123456".to_string(),
            name: "Test Server".to_string(),
            owner_id: "999".to_string(),
            icon_url: None,
            banner_url: None,
            description: Some("A test server".to_string()),
            vanity_url: None,
            verification_level: 0,
            explicit_content_filter: 0,
            default_notifications: 0,
            system_channel_id: None,
            rules_channel_id: None,
            created_at: now,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Test Server"));
        assert!(json.contains("123456"));
    }
}
