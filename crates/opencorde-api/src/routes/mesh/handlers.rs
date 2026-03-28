//! # Handlers: Mesh Federation
//! Route handler functions for mesh peer management.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use opencorde_core::snowflake::SnowflakeGenerator;
use opencorde_db::repos::mesh_peer_repo;

use super::types::{AddPeerRequest, MeshStatusResponse, PeerResponse};
use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

/// GET /api/v1/mesh/status — Get this server's mesh identity.
///
/// Returns the server's hostname, version, peer count, and user count.
/// Requires valid authentication token.
///
/// # Responses
/// - 200: MeshStatusResponse with server identity
/// - 401: Unauthorized (missing/invalid token)
#[tracing::instrument(skip(state, auth))]
pub async fn mesh_status(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<MeshStatusResponse>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "fetching mesh status");

    // Get active peer count
    let peers = mesh_peer_repo::list_active(&state.db)
        .await
        .map_err(ApiError::Database)?;
    let peers_count = peers.len() as i64;

    // Get actual user count from the database
    let users_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let response = MeshStatusResponse {
        hostname: state.config.mesh_hostname.clone(),
        version: "0.1.0".to_string(),
        peers_count,
        users_count,
    };

    tracing::info!(
        peers_count = peers_count,
        users_count = users_count,
        "mesh status returned"
    );

    Ok(Json(response))
}

/// GET /api/v1/mesh/peers — List all peer servers.
///
/// Returns all peers regardless of status (pending, active, suspended).
/// Requires valid authentication token.
///
/// # Responses
/// - 200: Array of PeerResponse
/// - 401: Unauthorized
#[tracing::instrument(skip(state, auth))]
pub async fn list_peers(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<PeerResponse>>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "listing all mesh peers");

    let peers = mesh_peer_repo::list_all(&state.db)
        .await
        .map_err(ApiError::Database)?;

    let count = peers.len();
    let responses = peers
        .into_iter()
        .map(|peer| {
            let status_str = match peer.status {
                0 => "pending",
                1 => "active",
                2 => "suspended",
                _ => "unknown",
            };

            PeerResponse {
                id: peer.id.to_string(),
                hostname: peer.hostname,
                public_key: peer.public_key,
                status: status_str.to_string(),
                last_seen_at: peer.last_seen_at.map(|dt| dt.to_rfc3339()),
                created_at: peer.created_at.to_rfc3339(),
            }
        })
        .collect();

    tracing::info!(count = count, "mesh peers listed");
    Ok(Json(responses))
}

/// POST /api/v1/mesh/peers — Register a new peer server.
///
/// Creates a pending peer entry. The peer will transition to "active" once
/// a successful handshake is performed.
///
/// Requires valid authentication token.
///
/// # Request Body
/// ```json
/// { "hostname": "other-server.com" }
/// ```
///
/// # Responses
/// - 201: PeerResponse with the new peer (status="pending")
/// - 400: Invalid request (missing hostname)
/// - 401: Unauthorized
/// - 409: Conflict (hostname already registered)
#[tracing::instrument(skip(state, auth))]
pub async fn add_peer(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<AddPeerRequest>,
) -> Result<(StatusCode, Json<PeerResponse>), ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        hostname = %req.hostname,
        "adding new mesh peer"
    );

    if req.hostname.is_empty() {
        return Err(ApiError::BadRequest("hostname is required".into()));
    }

    // Check if peer already exists
    if let Ok(Some(_)) = mesh_peer_repo::get_by_hostname(&state.db, &req.hostname).await {
        tracing::warn!(
            hostname = %req.hostname,
            "peer already registered"
        );
        return Err(ApiError::Conflict(
            "peer with this hostname already exists".into(),
        ));
    }

    // Generate Snowflake ID for the new peer
    let mut generator = SnowflakeGenerator::new(1, 1);
    let peer_id = generator.next_id();

    // Register peer with empty public key (will be filled during handshake)
    let peer = mesh_peer_repo::register_peer(&state.db, peer_id, &req.hostname, "")
        .await
        .map_err(ApiError::Database)?;

    let response = PeerResponse {
        id: peer.id.to_string(),
        hostname: peer.hostname,
        public_key: peer.public_key,
        status: "pending".to_string(),
        last_seen_at: None,
        created_at: peer.created_at.to_rfc3339(),
    };

    tracing::info!(peer_id = peer.id, "mesh peer added successfully");
    Ok((StatusCode::CREATED, Json(response)))
}

/// DELETE /api/v1/mesh/peers/{id} — Remove a peer server.
///
/// Removes a peer from the registry. Active connections are not affected
/// (they must be closed separately).
///
/// Requires valid authentication token.
///
/// # Path Parameters
/// - `id`: Peer Snowflake ID (as string)
///
/// # Responses
/// - 204: No content (peer deleted)
/// - 401: Unauthorized
/// - 404: Not found
#[tracing::instrument(skip(state, auth))]
pub async fn remove_peer(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        peer_id = %id,
        "removing mesh peer"
    );

    let peer_id: i64 = id
        .parse()
        .map_err(|_| ApiError::BadRequest("invalid peer ID".into()))?;

    let deleted = mesh_peer_repo::delete_peer(&state.db, peer_id.into())
        .await
        .map_err(ApiError::Database)?;

    if !deleted {
        tracing::warn!(peer_id = peer_id, "peer not found for deletion");
        return Err(ApiError::NotFound("peer not found".into()));
    }

    tracing::info!(peer_id = peer_id, "mesh peer removed");
    Ok(StatusCode::NO_CONTENT)
}
