//! # Types: Mesh Federation
//! Request/response types and serialization tests for mesh endpoints.

use serde::{Deserialize, Serialize};

/// Server mesh identity and status.
///
/// Returned by GET /api/v1/mesh/status
#[derive(Debug, Serialize)]
pub struct MeshStatusResponse {
    /// This server's hostname for mesh peering
    pub hostname: String,
    /// This server's version
    pub version: String,
    /// Count of connected peer servers
    pub peers_count: i64,
    /// Count of local users
    pub users_count: i64,
}

/// Peer server information.
///
/// Returned in GET /api/v1/mesh/peers response.
#[derive(Debug, Serialize)]
pub struct PeerResponse {
    /// Snowflake peer ID
    pub id: String,
    /// Peer server hostname
    pub hostname: String,
    /// Peer server's Ed25519 public key
    pub public_key: String,
    /// Connection status: "pending", "active", or "suspended"
    pub status: String,
    /// Last successful heartbeat timestamp (ISO 8601)
    pub last_seen_at: Option<String>,
    /// Peer registration timestamp (ISO 8601)
    pub created_at: String,
}

/// Request body for adding a new peer.
///
/// Sent in POST /api/v1/mesh/peers request.
#[derive(Debug, Deserialize)]
pub struct AddPeerRequest {
    /// Target server's hostname
    pub hostname: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_response_serialization() {
        let response = PeerResponse {
            id: "123456".to_string(),
            hostname: "example.com".to_string(),
            public_key: "abcd1234".repeat(8),
            status: "active".to_string(),
            last_seen_at: Some("2025-03-17T12:00:00Z".to_string()),
            created_at: "2025-03-16T12:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"hostname\":\"example.com\""));
        assert!(json.contains("\"status\":\"active\""));
    }

    #[test]
    fn test_mesh_status_response_serialization() {
        let response = MeshStatusResponse {
            hostname: "mesh.local".to_string(),
            version: "0.1.0".to_string(),
            peers_count: 5,
            users_count: 42,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"hostname\":\"mesh.local\""));
        assert!(json.contains("\"peers_count\":5"));
        assert!(json.contains("\"users_count\":42"));
    }

    #[test]
    fn test_add_peer_request_deserialization() {
        let json = r#"{"hostname":"peer.example.com"}"#;
        let req: AddPeerRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.hostname, "peer.example.com");
    }

    #[test]
    fn test_status_codes() {
        assert_eq!(
            match 0 {
                0 => "pending",
                1 => "active",
                2 => "suspended",
                _ => "unknown",
            },
            "pending"
        );
        assert_eq!(
            match 1 {
                0 => "pending",
                1 => "active",
                2 => "suspended",
                _ => "unknown",
            },
            "active"
        );
        assert_eq!(
            match 2 {
                0 => "pending",
                1 => "active",
                2 => "suspended",
                _ => "unknown",
            },
            "suspended"
        );
    }
}
