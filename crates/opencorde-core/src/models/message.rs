//! # Model: Message
//! Chat messages with attachments.

use crate::snowflake::Snowflake;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Chat message: text content, author, and attachments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message ID
    pub id: Snowflake,
    /// Channel this message was sent in
    pub channel_id: Snowflake,
    /// User who sent the message
    pub author_id: Snowflake,
    /// Message content
    pub content: String,
    /// Attached files
    pub attachments: Vec<Attachment>,
    /// Last edit timestamp (None if unedited)
    pub edited_at: Option<DateTime<Utc>>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// File attachment: metadata and URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// Unique attachment ID
    pub id: Snowflake,
    /// Original filename
    pub filename: String,
    /// File size in bytes
    pub size: i64,
    /// MIME type
    pub content_type: String,
    /// Download URL
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let message = Message {
            id: Snowflake::new(1000),
            channel_id: Snowflake::new(500),
            author_id: Snowflake::new(200),
            content: "Hello, world!".to_string(),
            attachments: vec![],
            edited_at: None,
            created_at: Utc::now(),
        };

        assert_eq!(message.content, "Hello, world!");
        assert!(message.attachments.is_empty());
        assert!(message.edited_at.is_none());
    }

    #[test]
    fn test_message_with_attachments() {
        let attachment = Attachment {
            id: Snowflake::new(2000),
            filename: "image.png".to_string(),
            size: 102400,
            content_type: "image/png".to_string(),
            url: "https://cdn.example.com/image.png".to_string(),
        };

        let message = Message {
            id: Snowflake::new(1001),
            channel_id: Snowflake::new(500),
            author_id: Snowflake::new(200),
            content: "Check out this image:".to_string(),
            attachments: vec![attachment],
            edited_at: None,
            created_at: Utc::now(),
        };

        assert_eq!(message.attachments.len(), 1);
        assert_eq!(message.attachments[0].filename, "image.png");
    }

    #[test]
    fn test_edited_message() {
        let now = Utc::now();
        let message = Message {
            id: Snowflake::new(1002),
            channel_id: Snowflake::new(500),
            author_id: Snowflake::new(200),
            content: "Edited content".to_string(),
            attachments: vec![],
            edited_at: Some(now),
            created_at: now,
        };

        assert!(message.edited_at.is_some());
    }

    #[test]
    fn test_attachment_metadata() {
        let attachment = Attachment {
            id: Snowflake::new(3000),
            filename: "document.pdf".to_string(),
            size: 1048576,
            content_type: "application/pdf".to_string(),
            url: "https://cdn.example.com/document.pdf".to_string(),
        };

        assert_eq!(attachment.size, 1048576);
        assert_eq!(attachment.content_type, "application/pdf");
    }

    #[test]
    fn test_message_serialization() {
        let message = Message {
            id: Snowflake::new(1003),
            channel_id: Snowflake::new(500),
            author_id: Snowflake::new(200),
            content: "Serialization test".to_string(),
            attachments: vec![],
            edited_at: None,
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, message.id);
        assert_eq!(deserialized.content, message.content);
    }

    #[test]
    fn test_attachment_serialization() {
        let attachment = Attachment {
            id: Snowflake::new(4000),
            filename: "test.txt".to_string(),
            size: 1024,
            content_type: "text/plain".to_string(),
            url: "https://cdn.example.com/test.txt".to_string(),
        };

        let json = serde_json::to_string(&attachment).unwrap();
        let deserialized: Attachment = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.filename, attachment.filename);
        assert_eq!(deserialized.size, attachment.size);
    }

    #[test]
    fn test_multiple_attachments() {
        let attachments = vec![
            Attachment {
                id: Snowflake::new(5000),
                filename: "file1.txt".to_string(),
                size: 512,
                content_type: "text/plain".to_string(),
                url: "https://cdn.example.com/file1.txt".to_string(),
            },
            Attachment {
                id: Snowflake::new(5001),
                filename: "file2.txt".to_string(),
                size: 1024,
                content_type: "text/plain".to_string(),
                url: "https://cdn.example.com/file2.txt".to_string(),
            },
        ];

        let message = Message {
            id: Snowflake::new(1004),
            channel_id: Snowflake::new(500),
            author_id: Snowflake::new(200),
            content: "Multiple attachments".to_string(),
            attachments,
            edited_at: None,
            created_at: Utc::now(),
        };

        assert_eq!(message.attachments.len(), 2);
    }
}
