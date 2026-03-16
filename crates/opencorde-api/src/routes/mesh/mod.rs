//! # Route: Mesh Federation
//! Admin endpoints for managing server-to-server peering.
//!
//! ## Endpoints
//! - GET /api/v1/mesh/status — This server's mesh identity
//! - GET /api/v1/mesh/peers — List all peered servers
//! - POST /api/v1/mesh/peers — Register a new peer (pending status)
//! - DELETE /api/v1/mesh/peers/{id} — Remove a peer
//!
//! ## Features
//! - Authentication required (AuthUser extractor)
//! - Peer status tracking (pending/active/suspended)
//! - Heartbeat timestamp monitoring
//! - Comprehensive structured logging
//!
//! ## Depends On
//! - axum (web framework)
//! - crate::middleware::auth::AuthUser (authentication)
//! - opencorde_db::repos::mesh_peer_repo (CRUD)
//! - opencorde_core::snowflake::SnowflakeGenerator (ID generation)
//! - crate::AppState (database + config)
//! - crate::error::ApiError (error handling)

mod handlers;
mod types;

use axum::{
    Router,
    routing::{delete, get},
};

use crate::AppState;

pub use handlers::{add_peer, list_peers, mesh_status, remove_peer};
pub use types::{AddPeerRequest, MeshStatusResponse, PeerResponse};

/// Build the mesh federation router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/mesh/status", get(mesh_status))
        .route("/api/v1/mesh/peers", get(list_peers).post(add_peer))
        .route("/api/v1/mesh/peers/{id}", delete(remove_peer))
}
