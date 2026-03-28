//! # Types: Federation API
//! Request/response shapes for server-to-server federation endpoints.

use serde::{Deserialize, Serialize};

/// Response: this server's public identity.
/// Returned by GET /api/v1/federation/identity — no auth required.
#[derive(Debug, Serialize)]
pub struct IdentityResponse {
    /// Server hostname (e.g. "chat.example.com")
    pub hostname: String,
    /// Ed25519 public key, hex-encoded (64 chars)
    pub public_key: String,
    /// Human-readable server name
    pub server_name: String,
    /// Software version
    pub version: &'static str,
}

/// Request body for POST /api/v1/federation/introduce
/// A remote server calls this to register itself with us.
#[derive(Debug, Deserialize)]
pub struct IntroduceRequest {
    /// Calling server's hostname
    pub hostname: String,
    /// Calling server's Ed25519 public key (hex, 64 chars)
    pub public_key: String,
    /// Human-readable name
    pub server_name: String,
    /// Unix timestamp (seconds) — included in signature to prevent replay
    pub timestamp: i64,
    /// Ed25519 signature over `"{hostname}:{timestamp}"` using the server's private key
    pub signature: String,
}

/// Response to a successful introduce handshake.
#[derive(Debug, Serialize)]
pub struct IntroduceResponse {
    /// Whether this is a new peer (true) or we already knew them (false)
    pub accepted: bool,
    /// Our hostname — so the caller can confirm they reached the right server
    pub hostname: String,
    /// Our public key — so the caller can store and verify our future events
    pub public_key: String,
}

/// A signed federated event payload.
/// Sent to POST /api/v1/federation/events by a peered server.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FederatedEvent {
    /// Originating server hostname
    pub origin: String,
    /// Ed25519 public key of origin server (hex)
    pub origin_pubkey: String,
    /// Unix timestamp — prevents replay
    pub timestamp: i64,
    /// Event type: "MessageCreate", "PresenceUpdate", etc.
    pub event_type: String,
    /// JSON payload specific to event_type
    pub payload: serde_json::Value,
    /// Signature over `"{origin}:{timestamp}:{event_type}:{payload_json}"`
    pub signature: String,
}

/// Federated message payload (inside FederatedEvent.payload for MessageCreate).
#[derive(Debug, Deserialize, Serialize)]
pub struct FederatedMessage {
    /// Channel ID on the origin server
    pub channel_id: String,
    /// Message content
    pub content: String,
    /// Author's username on their home server
    pub author_username: String,
    /// Author's home server hostname
    pub author_server: String,
    /// Globally unique message ID (snowflake from origin server)
    pub message_id: String,
}

/// Direct message payload for cross-server DMs (FederatedEvent.payload for FederatedDMCreate).
///
/// Sent when a user on server A sends a DM to a user on server B.
/// Server B finds the local recipient and delivers the message to their inbox.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FederatedDMPayload {
    /// Recipient address on this server ("username" part — the server is known from the event origin)
    pub recipient_username: String,
    /// Sender's full address on the origin server, e.g. "bob@server1.com"
    pub sender_address: String,
    /// Message content
    pub content: String,
    /// Snowflake message ID assigned by the origin server
    pub message_id: String,
}

/// Response for GET /api/v1/federation/users/{username}.
///
/// Allows remote servers to verify a user exists before opening a federated DM.
#[derive(Debug, Serialize)]
pub struct UserLookupResponse {
    /// Username (unique on this server)
    pub username: String,
    /// Display name (may differ from username)
    pub display_name: String,
    /// Server hostname — so the caller can confirm they reached the right server
    pub server: String,
}
