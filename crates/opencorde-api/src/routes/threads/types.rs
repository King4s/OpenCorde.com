//! # Thread Route Types
//! Request and response types for thread endpoints.
//!
//! ## Depends On
//! - serde (serialization)
//! - chrono (timestamps)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Thread response body.
#[derive(Debug, Serialize, Clone)]
pub struct ThreadResponse {
    pub id: String,
    pub channel_id: String,
    pub parent_msg_id: Option<String>,
    pub name: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub last_msg_at: DateTime<Utc>,
    pub msg_count: i32,
}

/// Request to create a thread.
#[derive(Debug, Deserialize)]
pub struct CreateThreadRequest {
    pub name: Option<String>,
}

/// Request to send a message in a thread.
#[derive(Debug, Deserialize)]
pub struct SendThreadMessageRequest {
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_response_serialization() {
        let now = Utc::now();
        let response = ThreadResponse {
            id: "999888777".to_string(),
            channel_id: "555666".to_string(),
            parent_msg_id: Some("123456".to_string()),
            name: "Discussion".to_string(),
            created_by: "111222".to_string(),
            created_at: now,
            last_msg_at: now,
            msg_count: 5,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("999888777"));
        assert!(json.contains("Discussion"));
        assert!(json.contains("123456"));
    }
}
