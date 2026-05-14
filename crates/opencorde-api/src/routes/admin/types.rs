//! # Admin Types
//! Data structures for admin endpoints.

use serde::{Deserialize, Serialize};

/// Instance statistics response.
#[derive(Debug, Serialize)]
pub struct InstanceStats {
    /// Total number of users
    pub total_users: i64,
    /// Total number of servers
    pub total_servers: i64,
    /// Total number of messages
    pub total_messages: i64,
    /// Total number of channels
    pub total_channels: i64,
    /// Active voice sessions
    pub active_voice_sessions: i64,
    /// PostgreSQL database size in bytes
    pub db_size_bytes: i64,
    /// Total size of all uploaded files in bytes
    pub attachment_storage_bytes: i64,
    /// Total number of uploaded files
    pub attachment_count: i64,
    /// LiveKit local and public endpoint health.
    pub livekit_health: LiveKitHealth,
}

/// LiveKit endpoint health summary for admin dashboards.
#[derive(Debug, Serialize)]
pub struct LiveKitHealth {
    /// True when both configured endpoints responded with OK.
    pub ok: bool,
    /// Internal LiveKit endpoint health.
    pub local: LiveKitEndpointHealth,
    /// Public LiveKit endpoint health as reached through the reverse proxy.
    pub public: LiveKitEndpointHealth,
}

/// Single LiveKit endpoint health result.
#[derive(Debug, Serialize)]
pub struct LiveKitEndpointHealth {
    /// URL checked after translating ws/wss to http/https.
    pub url: String,
    /// True when the endpoint returns HTTP 200 with body "OK".
    pub ok: bool,
    /// HTTP status code, if a response was received.
    pub status: Option<u16>,
    /// Request latency in milliseconds.
    pub latency_ms: Option<u128>,
    /// Short failure detail, if the endpoint did not pass.
    pub error: Option<String>,
}

/// User row for admin listing.
#[derive(Debug, Serialize)]
pub struct AdminUserRow {
    /// Snowflake user ID
    pub id: String,
    /// Username
    pub username: String,
    /// Email address
    pub email: String,
    /// Account creation time
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Server row for admin listing.
#[derive(Debug, Serialize)]
pub struct AdminServerRow {
    /// Snowflake server ID
    pub id: String,
    /// Server name
    pub name: String,
    /// Server owner ID
    pub owner_id: i64,
    /// Current member count
    pub member_count: i32,
    /// Server creation time
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Pagination query parameters.
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    /// Items per page (default: 50)
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Offset for pagination (default: 0)
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limit() {
        assert_eq!(default_limit(), 50);
    }

    #[test]
    fn test_pagination_query_default() {
        let query: PaginationQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(query.limit, 50);
        assert_eq!(query.offset, 0);
    }

    #[test]
    fn test_pagination_query_custom() {
        let query: PaginationQuery =
            serde_json::from_str("{\"limit\": 25, \"offset\": 10}").unwrap();
        assert_eq!(query.limit, 25);
        assert_eq!(query.offset, 10);
    }
}
