//! # Message Route Types
//! Request and response types for message endpoints.
//!
//! ## Depends On
//! - serde (serialization)
//! - chrono (timestamps)
//! - serde_json (JSON values)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Request body for sending a message.
#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    /// Message content (1-4000 characters)
    pub content: String,
}

/// Request body for editing a message.
#[derive(Debug, Deserialize)]
pub struct EditMessageRequest {
    /// Updated message content (1-4000 characters)
    pub content: String,
}

/// Query parameters for listing messages (cursor-based pagination).
#[derive(Debug, Deserialize)]
pub struct MessageQuery {
    /// Optional cursor: fetch messages with ID less than this value
    pub before: Option<String>,
    /// Optional cursor: fetch messages with ID greater than this value
    pub after: Option<String>,
    /// Message limit (1-100, defaults to 50)
    pub limit: Option<i64>,
}

/// Message response body.
#[derive(Debug, Serialize, Clone)]
pub struct MessageResponse {
    /// Snowflake message ID
    pub id: String,
    /// Snowflake channel ID
    pub channel_id: String,
    /// Snowflake author user ID
    pub author_id: String,
    /// Message content text
    pub content: String,
    /// Array of attachments (JSON array)
    pub attachments: serde_json::Value,
    /// Timestamp when message was last edited (None if unedited)
    pub edited_at: Option<DateTime<Utc>>,
    /// Timestamp when message was created
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_message_request_deserialization() {
        let json = r#"{"content":"Hello, world!"}"#;
        let req: SendMessageRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "Hello, world!");
    }

    #[test]
    fn test_edit_message_request_deserialization() {
        let json = r#"{"content":"Updated message"}"#;
        let req: EditMessageRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "Updated message");
    }

    #[test]
    fn test_message_query_with_before() {
        let json = r#"{"before":"123456","limit":25}"#;
        let query: MessageQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.before, Some("123456".to_string()));
        assert_eq!(query.after, None);
        assert_eq!(query.limit, Some(25));
    }

    #[test]
    fn test_message_query_empty() {
        let json = r#"{}"#;
        let query: MessageQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.before, None);
        assert_eq!(query.after, None);
        assert_eq!(query.limit, None);
    }

    #[test]
    fn test_message_response_serialization() {
        let now = Utc::now();
        let response = MessageResponse {
            id: "999888777".to_string(),
            channel_id: "555666".to_string(),
            author_id: "111222".to_string(),
            content: "Test message".to_string(),
            attachments: serde_json::json!([]),
            edited_at: None,
            created_at: now,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("999888777"));
        assert!(json.contains("Test message"));
    }

    #[test]
    fn test_message_response_with_edit() {
        let now = Utc::now();
        let edited = now + chrono::Duration::seconds(60);
        let response = MessageResponse {
            id: "999888777".to_string(),
            channel_id: "555666".to_string(),
            author_id: "111222".to_string(),
            content: "Edited message".to_string(),
            attachments: serde_json::json!([]),
            edited_at: Some(edited),
            created_at: now,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Edited message"));
        assert!(json.contains("edited_at"));
    }
}
