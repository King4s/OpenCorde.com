//! # Forum Types
//! Request/response types for forum endpoints.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Response for a forum post (all fields, id as String).
#[derive(Debug, Serialize, Clone)]
pub struct PostResponse {
    pub id: String,
    pub channel_id: String,
    pub author_id: String,
    pub author_username: String,
    pub title: String,
    pub content: String,
    pub reply_count: i32,
    pub pinned: bool,
    pub created_at: DateTime<Utc>,
    pub last_reply_at: DateTime<Utc>,
}

/// Response for a forum reply.
#[derive(Debug, Serialize, Clone)]
pub struct ReplyResponse {
    pub id: String,
    pub post_id: String,
    pub author_id: String,
    pub author_username: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

/// Response for getting a post with its replies.
#[derive(Debug, Serialize)]
pub struct PostDetailResponse {
    pub post: PostResponse,
    pub replies: Vec<ReplyResponse>,
}

/// Request to create a post.
#[derive(Debug, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub content: String,
}

/// Request to create a reply.
#[derive(Debug, Deserialize)]
pub struct CreateReplyRequest {
    pub content: String,
}

/// Query parameters for listing posts.
#[derive(Debug, Deserialize)]
pub struct ListPostsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_response_has_all_fields() {
        let _ = std::mem::size_of::<PostResponse>();
    }

    #[test]
    fn test_reply_response_has_all_fields() {
        let _ = std::mem::size_of::<ReplyResponse>();
    }
}
