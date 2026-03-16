//! # Mesh Federation Protocol
//! Types for server-to-server communication in IRC-inspired federation.
//!
//! ## Protocol Overview
//! 1. Server A sends PEER_REQUEST to Server B with its public key + hostname
//! 2. Server B validates, sends PEER_ACCEPT with its own key
//! 3. Both servers exchange events via MESH_EVENT messages
//! 4. Heartbeats every 60 seconds maintain connections
//!
//! ## Serialization
//! All variants serialize with `type` tag for routing.
//!
//! ## Depends On
//! - serde — Serialization/deserialization
//! - serde_json — JSON payload

use serde::{Deserialize, Serialize};

/// Messages exchanged between peered mesh servers.
///
/// Uses tagged enum serialization: each variant produces JSON with "type" field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MeshMessage {
    /// Initial peering request from one server to another.
    PeerRequest {
        /// Requesting server's hostname (e.g., "mesh.example.com")
        hostname: String,
        /// Requesting server's Ed25519 public key (hex-encoded, 64 chars)
        public_key: String,
        /// Human-readable server name
        server_name: String,
        /// Server version (e.g., "0.1.0")
        version: String,
    },

    /// Accept peering request.
    PeerAccept {
        /// Accepting server's hostname
        hostname: String,
        /// Accepting server's Ed25519 public key
        public_key: String,
        /// Human-readable server name
        server_name: String,
    },

    /// Reject peering request.
    PeerReject {
        /// Human-readable reason for rejection
        reason: String,
    },

    /// Heartbeat ping (sent every 60 seconds).
    Heartbeat {
        /// Unix timestamp (seconds)
        timestamp: i64,
        /// Number of connected users on sending server
        user_count: u64,
    },

    /// Heartbeat acknowledgement.
    HeartbeatAck {
        /// Echo of heartbeat timestamp
        timestamp: i64,
    },

    /// Federated event from another server.
    Event {
        /// Originating server hostname
        origin_server: String,
        /// Event category (e.g., "message", "presence", "typing")
        event_type: String,
        /// Arbitrary event data as JSON
        payload: serde_json::Value,
    },

    /// Request user information by public key.
    UserLookup {
        /// User's Ed25519 public key
        public_key: String,
    },

    /// User information response.
    UserInfo {
        /// User's Ed25519 public key
        public_key: String,
        /// User's display name
        username: String,
        /// Optional avatar URL
        avatar_url: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_messages_serialization() {
        let peer_request = MeshMessage::PeerRequest {
            hostname: "mesh.example.com".to_string(),
            public_key: "abcd1234".repeat(8),
            server_name: "Example Mesh".to_string(),
            version: "0.1.0".to_string(),
        };

        let json = serde_json::to_string(&peer_request).unwrap();
        assert!(json.contains("\"type\":\"PeerRequest\""));
        assert!(json.contains("\"hostname\":\"mesh.example.com\""));
        let deserialized: MeshMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, MeshMessage::PeerRequest { .. }));

        let peer_accept = MeshMessage::PeerAccept {
            hostname: "other.mesh".to_string(),
            public_key: "def5678".repeat(9 + 1)[..64].to_string(),
            server_name: "Other Mesh".to_string(),
        };

        let json = serde_json::to_string(&peer_accept).unwrap();
        assert!(json.contains("\"type\":\"PeerAccept\""));
        let deserialized: MeshMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, MeshMessage::PeerAccept { .. }));

        let peer_reject = MeshMessage::PeerReject {
            reason: "version mismatch".to_string(),
        };

        let json = serde_json::to_string(&peer_reject).unwrap();
        assert!(json.contains("\"type\":\"PeerReject\""));
        let deserialized: MeshMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, MeshMessage::PeerReject { .. }));
    }

    #[test]
    fn test_heartbeat_messages_serialization() {
        let heartbeat = MeshMessage::Heartbeat {
            timestamp: 1705000000,
            user_count: 42,
        };

        let json = serde_json::to_string(&heartbeat).unwrap();
        assert!(json.contains("\"type\":\"Heartbeat\""));
        assert!(json.contains("1705000000"));
        let deserialized: MeshMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, MeshMessage::Heartbeat { .. }));

        let heartbeat_ack = MeshMessage::HeartbeatAck {
            timestamp: 1705000000,
        };
        let json = serde_json::to_string(&heartbeat_ack).unwrap();
        assert!(json.contains("\"type\":\"HeartbeatAck\""));
        let deserialized: MeshMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, MeshMessage::HeartbeatAck { .. }));
    }

    #[test]
    fn test_event_messages_serialization() {
        let payload = serde_json::json!({
            "channel_id": "123456",
            "content": "hello mesh"
        });

        let event = MeshMessage::Event {
            origin_server: "mesh.example.com".to_string(),
            event_type: "message".to_string(),
            payload,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"Event\""));
        assert!(json.contains("message"));
        let deserialized: MeshMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, MeshMessage::Event { .. }));
    }

    #[test]
    fn test_user_lookup_messages_serialization() {
        let user_lookup = MeshMessage::UserLookup {
            public_key: "abc123".repeat(10 + 2)[..64].to_string(),
        };

        let json = serde_json::to_string(&user_lookup).unwrap();
        assert!(json.contains("\"type\":\"UserLookup\""));
        let deserialized: MeshMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, MeshMessage::UserLookup { .. }));

        let user_info = MeshMessage::UserInfo {
            public_key: "def456".repeat(10 + 2)[..64].to_string(),
            username: "mesh_user".to_string(),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
        };

        let json = serde_json::to_string(&user_info).unwrap();
        assert!(json.contains("\"type\":\"UserInfo\""));
        assert!(json.contains("mesh_user"));
        let deserialized: MeshMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, MeshMessage::UserInfo { .. }));
    }

    #[test]
    fn test_all_variants_round_trip() {
        let messages = vec![
            MeshMessage::PeerRequest {
                hostname: "test.local".to_string(),
                public_key: "aa".repeat(32),
                server_name: "Test".to_string(),
                version: "1.0".to_string(),
            },
            MeshMessage::PeerAccept {
                hostname: "test2.local".to_string(),
                public_key: "bb".repeat(32),
                server_name: "Test2".to_string(),
            },
            MeshMessage::PeerReject {
                reason: "denied".to_string(),
            },
            MeshMessage::Heartbeat {
                timestamp: 0,
                user_count: 1,
            },
            MeshMessage::HeartbeatAck { timestamp: 0 },
            MeshMessage::Event {
                origin_server: "origin".to_string(),
                event_type: "test".to_string(),
                payload: serde_json::json!({}),
            },
            MeshMessage::UserLookup {
                public_key: "cc".repeat(32),
            },
            MeshMessage::UserInfo {
                public_key: "dd".repeat(32),
                username: "user".to_string(),
                avatar_url: None,
            },
        ];

        for original in messages {
            let json = serde_json::to_string(&original).unwrap();
            let deserialized: MeshMessage = serde_json::from_str(&json).unwrap();
            let json2 = serde_json::to_string(&deserialized).unwrap();
            assert_eq!(json, json2, "Round trip failed for {:?}", original);
        }
    }
}
