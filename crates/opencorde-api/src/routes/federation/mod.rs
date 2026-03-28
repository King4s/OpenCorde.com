//! # Route: Federation API
//! Server-to-server endpoints for mesh federation.
//!
//! ## Public Endpoints (no auth)
//! - GET /api/v1/federation/identity — This server's hostname + Ed25519 public key
//!
//! ## Authenticated by Signature (not JWT)
//! - POST /api/v1/federation/introduce — Peer handshake + registration
//! - POST /api/v1/federation/events    — Receive signed events from active peers

mod handlers;
mod types;

pub use handlers::{get_identity, introduce, lookup_user, receive_event};
pub use types::FederatedEvent;

use axum::{Router, routing::{get, post}};
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/federation/identity", get(get_identity))
        .route("/api/v1/federation/introduce", post(introduce))
        .route("/api/v1/federation/events", post(receive_event))
        // User lookup: allows remote servers to verify a user exists before opening a federated DM
        .route("/api/v1/federation/users/{username}", get(lookup_user))
}
