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
    /// Optional Snowflake ID of the message being replied to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to_id: Option<String>,
    /// Optional array of attachment objects to attach to this message
    #[serde(default)]
    pub attachments: Option<serde_json::Value>,
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

/// Context of a message being replied to (minimal data for display).
#[derive(Debug, Serialize, Clone)]
pub struct ReplyContextResponse {
    /// Snowflake message ID of the replied-to message
    pub id: String,
    /// Username of the author of the replied-to message
    pub author_username: String,
    /// Content preview of the replied-to message (first 100 chars)
    pub content: String,
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
    /// Author's username
    pub author_username: String,
    /// Message content text
    pub content: String,
    /// Array of attachments (JSON array)
    pub attachments: serde_json::Value,
    /// Timestamp when message was last edited (None if unedited)
    pub edited_at: Option<DateTime<Utc>>,
    /// Timestamp when message was created
    pub created_at: DateTime<Utc>,
    /// Snowflake ID of the message this is replying to (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to_id: Option<String>,
    /// Inline reply context (author + content preview) for the replied-to message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<ReplyContextResponse>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_message_request_deserialization() {
        let json = r#"{"content":"Hello, world!"}"#;
        let req: SendMessageRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "Hello, world!");
        assert!(req.reply_to_id.is_none());
    }

    #[test]
    fn test_send_message_request_with_reply() {
        let json = r#"{"content":"Reply text","reply_to_id":"123456789"}"#;
        let req: SendMessageRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "Reply text");
        assert_eq!(req.reply_to_id, Some("123456789".to_string()));
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
            author_username: "testuser".to_string(),
            content: "Test message".to_string(),
            attachments: serde_json::json!([]),
            edited_at: None,
            created_at: now,
            reply_to_id: None,
            reply_to: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("999888777"));
        assert!(json.contains("Test message"));
        assert!(json.contains("testuser"));
        assert!(!json.contains("reply_to_id"));
    }

    #[test]
    fn test_message_response_with_edit() {
        let now = Utc::now();
        let edited = now + chrono::Duration::seconds(60);
        let response = MessageResponse {
            id: "999888777".to_string(),
            channel_id: "555666".to_string(),
            author_id: "111222".to_string(),
            author_username: "testuser".to_string(),
            content: "Edited message".to_string(),
            attachments: serde_json::json!([]),
            edited_at: Some(edited),
            created_at: now,
            reply_to_id: None,
            reply_to: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Edited message"));
        assert!(json.contains("edited_at"));
        assert!(json.contains("testuser"));
    }

    #[test]
    fn test_message_response_with_reply() {
        let now = Utc::now();
        let response = MessageResponse {
            id: "999888777".to_string(),
            channel_id: "555666".to_string(),
            author_id: "111222".to_string(),
            author_username: "testuser".to_string(),
            content: "Reply message".to_string(),
            attachments: serde_json::json!([]),
            edited_at: None,
            created_at: now,
            reply_to_id: Some("123456".to_string()),
            reply_to: Some(ReplyContextResponse {
                id: "123456".to_string(),
                author_username: "origauthor".to_string(),
                content: "Original content".to_string(),
            }),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("reply_to_id"));
        assert!(json.contains("123456"));
        assert!(json.contains("reply_to"));
        assert!(json.contains("origauthor"));
    }
}
