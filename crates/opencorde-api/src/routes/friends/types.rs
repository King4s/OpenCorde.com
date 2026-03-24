//! Type definitions for friend operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Relationship response type.
#[derive(Debug, Serialize)]
pub struct RelationshipResponse {
    pub id: String,
    pub from_user: String,
    pub to_user: String,
    pub status: String,
    pub other_username: String,
    pub other_avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Pending requests grouped by incoming and outgoing.
#[derive(Debug, Serialize)]
pub struct PendingResponse {
    pub incoming: Vec<RelationshipResponse>,
    pub outgoing: Vec<RelationshipResponse>,
}

/// Request body for sending a friend request or blocking a user.
#[derive(Debug, Deserialize)]
pub struct UserIdRequest {
    pub user_id: String, // snowflake string
}

/// User search result.
#[derive(Debug, Serialize)]
pub struct UserSearchResult {
    pub id: String,
    pub username: String,
    pub avatar_url: Option<String>,
}
