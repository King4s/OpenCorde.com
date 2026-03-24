//! # Repository: Messages
//! CRUD operations for channel messages.
//!
//! Supports cursor-based pagination (before/after) for efficient message retrieval.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake

mod crud;
mod query;

pub use crud::{create_message, update_content, delete_message};
pub use query::{
    get_by_id, list_by_channel, list_by_thread, get_reply_context,
};
pub use crate::repos::message_repo::crud::MessageRow;
pub use crate::repos::message_repo::query::ReplyContext;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::Value as JsonValue;

    #[test]
    fn test_message_row_creation() {
        let now = Utc::now();
        let row = MessageRow {
            id: 777888999,
            channel_id: 555666777,
            author_id: 111222333,
            content: "Hello, world!".to_string(),
            attachments: JsonValue::Array(Vec::new()),
            edited_at: None,
            created_at: now,
            author_username: "testuser".to_string(),
            reply_to_id: None,
            reply_author_username: None,
            reply_content_preview: None,
            thread_id: None,
        };

        assert_eq!(row.id, 777888999);
        assert_eq!(row.content, "Hello, world!");
        assert_eq!(row.author_username, "testuser");
        assert!(row.edited_at.is_none());
        assert!(row.reply_to_id.is_none());
        assert!(row.thread_id.is_none());
    }

    #[test]
    fn test_pagination_limit() {
        let limit = 200i64;
        let capped = std::cmp::min(limit, 100);
        assert_eq!(capped, 100);

        let limit = 50i64;
        let capped = std::cmp::min(limit, 100);
        assert_eq!(capped, 50);
    }
}
