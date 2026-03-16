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
    /// Snowflake server ID
    pub id: String,
    /// Server name (1-100 chars)
    pub name: String,
    /// Snowflake user ID of server owner
    pub owner_id: String,
    /// Server icon URL (optional)
    pub icon_url: Option<String>,
    /// Server description (optional)
    pub description: Option<String>,
    /// Server creation timestamp
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
    /// Optional new server name
    pub name: Option<String>,
    /// Optional new server description
    pub description: Option<String>,
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
            description: Some("A test server".to_string()),
            created_at: now,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Test Server"));
        assert!(json.contains("123456"));
    }
}
