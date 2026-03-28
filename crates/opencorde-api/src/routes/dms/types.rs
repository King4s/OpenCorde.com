//! # DM Types
//! Request and response types for DM endpoints.
//!
//! ## Depends On
//! - serde (serialization)
//! - chrono (timestamps)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Request to open a DM with a recipient.
///
/// For local users: set `recipient_id` to their snowflake ID.
/// For users on another server: set `recipient_address` to "username@hostname".
/// Exactly one of the two fields must be provided.
#[derive(Debug, Deserialize)]
pub struct OpenDmRequest {
    /// Local user's snowflake ID (for same-server DMs)
    pub recipient_id: Option<String>,
    /// Remote user address "username@hostname" (for cross-server DMs)
    pub recipient_address: Option<String>,
}

/// Response for a DM channel.
#[derive(Debug, Serialize)]
pub struct DmChannelResponse {
    pub id: String,
    pub other_user_id: String,
    pub other_username: String,
    pub last_read_id: String,
}

/// Response for a DM message.
#[derive(Debug, Serialize)]
pub struct DmMessageResponse {
    pub id: String,
    pub dm_id: String,
    pub author_id: String,
    pub author_username: String,
    pub content: String,
    pub attachments: serde_json::Value,
    pub edited_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Request to send a DM message.
#[derive(Debug, Deserialize)]
pub struct SendDmRequest {
    pub content: String,
}

/// Query parameters for listing DM messages.
#[derive(Debug, Deserialize)]
pub struct MessageListQuery {
    pub before: Option<String>,
    pub limit: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dm_channel_response_creation() {
        let resp = DmChannelResponse {
            id: "123".to_string(),
            other_user_id: "456".to_string(),
            other_username: "alice".to_string(),
            last_read_id: "789".to_string(),
        };

        assert_eq!(resp.id, "123");
        assert_eq!(resp.other_username, "alice");
    }

    #[test]
    fn test_dm_message_response_creation() {
        let now = Utc::now();
        let resp = DmMessageResponse {
            id: "111".to_string(),
            dm_id: "222".to_string(),
            author_id: "333".to_string(),
            author_username: "bob".to_string(),
            content: "Hello!".to_string(),
            attachments: serde_json::Value::Array(Vec::new()),
            edited_at: None,
            created_at: now,
        };

        assert_eq!(resp.content, "Hello!");
        assert_eq!(resp.author_username, "bob");
    }
}
