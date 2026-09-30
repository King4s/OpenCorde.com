//! Shared message helpers — row-to-response conversion and tests.

use opencorde_db::repos::message_repo;

use super::types::{MessageResponse, ReplyContextResponse};

/// Convert a MessageRow from the database into an API MessageResponse.
pub fn message_row_to_response(row: message_repo::MessageRow) -> MessageResponse {
    let reply_to = match (
        row.reply_to_id,
        row.reply_author_username,
        row.reply_content_preview,
    ) {
        (Some(id), Some(author), Some(content)) => Some(ReplyContextResponse {
            id: id.to_string(),
            author_username: author,
            content,
        }),
        _ => None,
    };
    MessageResponse {
        id: row.id.to_string(),
        channel_id: row.channel_id.to_string(),
        author_id: row.author_id.to_string(),
        author_username: row.author_username,
        content: row.content,
        attachments: row.attachments,
        edited_at: row.edited_at,
        created_at: row.created_at,
        reply_to_id: row.reply_to_id.map(|id| id.to_string()),
        reply_to,
        forwarded_from: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_row_to_response() {
        use chrono::Utc;
        use serde_json::json;

        let now = Utc::now();
        let row = message_repo::MessageRow {
            id: 777888999,
            channel_id: 555666777,
            author_id: 111222333,
            content: "Test message".to_string(),
            attachments: json!([]),
            edited_at: None,
            created_at: now,
            author_username: "testuser".to_string(),
            reply_to_id: None,
            reply_author_username: None,
            reply_content_preview: None,
            thread_id: None,
            forwarded_from_id: None,
        };

        let response = message_row_to_response(row);
        assert_eq!(response.id, "777888999");
        assert_eq!(response.channel_id, "555666777");
        assert_eq!(response.author_id, "111222333");
        assert_eq!(response.author_username, "testuser");
        assert_eq!(response.content, "Test message");
        assert!(response.edited_at.is_none());
        assert!(response.reply_to_id.is_none());
        assert!(response.reply_to.is_none());
    }

    #[test]
    fn test_message_row_to_response_with_reply() {
        use chrono::Utc;
        use serde_json::json;

        let now = Utc::now();
        let row = message_repo::MessageRow {
            id: 777888999,
            channel_id: 555666777,
            author_id: 111222333,
            content: "Reply message".to_string(),
            attachments: json!([]),
            edited_at: None,
            created_at: now,
            author_username: "testuser".to_string(),
            reply_to_id: Some(123456),
            reply_author_username: Some("originaluser".to_string()),
            reply_content_preview: Some("Original content".to_string()),
            thread_id: None,
            forwarded_from_id: None,
        };

        let response = message_row_to_response(row);
        assert_eq!(response.id, "777888999");
        assert_eq!(response.reply_to_id, Some("123456".to_string()));
        assert!(response.reply_to.is_some());
        let ctx = response.reply_to.unwrap();
        assert_eq!(ctx.author_username, "originaluser");
        assert_eq!(ctx.content, "Original content");
    }
}
