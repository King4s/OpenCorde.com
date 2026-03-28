//! # Handlers: Federation API
//! Server-to-server HTTP endpoints for mesh federation.
//!
//! ## Endpoints
//! - GET  /api/v1/federation/identity  — Public: returns this server's hostname + pubkey
//! - POST /api/v1/federation/introduce — Peer calls this to handshake and register
//! - POST /api/v1/federation/events    — Peer delivers a signed event to this server
//!
//! ## Security
//! All incoming POST requests must carry a valid Ed25519 signature.
//! Signatures are over a deterministic string including the timestamp,
//! which prevents replay attacks (we reject timestamps >5 min old).

use axum::{Json, extract::{Path, State}, http::StatusCode};
use opencorde_core::snowflake::SnowflakeGenerator;
use opencorde_db::repos::{dm_federated_repo, mesh_peer_repo, user_repo};

use crate::{AppState, error::ApiError, identity::ServerIdentity};
use super::types::*;

const REPLAY_WINDOW_SECS: i64 = 300; // 5 minutes

/// GET /api/v1/federation/users/:username
///
/// Public endpoint. Allows remote servers to confirm a user exists here
/// before opening a cross-server DM. Returns display info only — no private data.
pub async fn lookup_user(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<UserLookupResponse>, ApiError> {
    let user = user_repo::get_by_username(&state.db, &username)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound(format!("user '{}' not found", username)))?;

    Ok(Json(UserLookupResponse {
        username: user.username,
        display_name: user.status_message.unwrap_or_default(),
        server: state.config.mesh_hostname.clone(),
    }))
}

/// GET /api/v1/federation/identity
///
/// Public endpoint. Returns this server's cryptographic identity so
/// peers can verify our signatures and register us in their mesh_peers.
pub async fn get_identity(State(state): State<AppState>) -> Json<IdentityResponse> {
    Json(IdentityResponse {
        hostname: state.config.mesh_hostname.clone(),
        public_key: state.identity.public_key_hex.clone(),
        server_name: state.config.mesh_hostname.clone(),
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// POST /api/v1/federation/introduce
///
/// A remote server calls this to register itself with us.
/// We verify the signature, then upsert into mesh_peers as active.
///
/// Signature message: `"{hostname}:{timestamp}"`
pub async fn introduce(
    State(state): State<AppState>,
    Json(req): Json<IntroduceRequest>,
) -> Result<(StatusCode, Json<IntroduceResponse>), ApiError> {
    // Replay protection: reject timestamps older than 5 minutes
    let now = chrono::Utc::now().timestamp();
    if (now - req.timestamp).abs() > REPLAY_WINDOW_SECS {
        tracing::warn!(
            hostname = %req.hostname,
            timestamp = req.timestamp,
            "rejecting introduce: timestamp out of window"
        );
        return Err(ApiError::BadRequest("timestamp out of acceptable window".into()));
    }

    // Verify signature
    let signed_msg = format!("{}:{}", req.hostname, req.timestamp);
    if !ServerIdentity::verify(&req.public_key, signed_msg.as_bytes(), &req.signature) {
        tracing::warn!(
            hostname = %req.hostname,
            "rejecting introduce: invalid signature"
        );
        return Err(ApiError::Unauthorized);
    }

    // Upsert peer — update pubkey and activate if already exists, else insert
    let existing = mesh_peer_repo::get_by_hostname(&state.db, &req.hostname)
        .await
        .map_err(ApiError::Database)?;

    let is_new = existing.is_none();

    if let Some(peer) = existing {
        // Re-activate and update public key in case it changed
        sqlx::query(
            "UPDATE mesh_peers SET public_key = $1, status = 1, last_seen_at = NOW() WHERE id = $2"
        )
        .bind(&req.public_key)
        .bind(peer.id)
        .execute(&state.db)
        .await
        .map_err(ApiError::Database)?;

        tracing::info!(hostname = %req.hostname, "existing peer re-activated via introduce");
    } else {
        let mut sf_gen = SnowflakeGenerator::new(1, 1);
        let new_id = sf_gen.next_id();
        sqlx::query(
            "INSERT INTO mesh_peers (id, hostname, public_key, status, last_seen_at) \
             VALUES ($1, $2, $3, 1, NOW())"
        )
        .bind(new_id.as_i64())
        .bind(&req.hostname)
        .bind(&req.public_key)
        .execute(&state.db)
        .await
        .map_err(ApiError::Database)?;

        tracing::info!(hostname = %req.hostname, "new peer registered via introduce");
    }

    let status = if is_new { StatusCode::CREATED } else { StatusCode::OK };
    Ok((status, Json(IntroduceResponse {
        accepted: true,
        hostname: state.config.mesh_hostname.clone(),
        public_key: state.identity.public_key_hex.clone(),
    })))
}

/// POST /api/v1/federation/events
///
/// A peered server delivers a signed event. We verify:
/// 1. The origin is a known active peer
/// 2. The signature is valid for that peer's stored public key
/// 3. The timestamp is within the replay window
///
/// Then we process the event locally.
pub async fn receive_event(
    State(state): State<AppState>,
    Json(event): Json<FederatedEvent>,
) -> Result<StatusCode, ApiError> {
    // 1. Replay protection
    let now = chrono::Utc::now().timestamp();
    if (now - event.timestamp).abs() > REPLAY_WINDOW_SECS {
        return Err(ApiError::BadRequest("timestamp out of window".into()));
    }

    // 2. Look up peer's stored public key
    let peer = mesh_peer_repo::get_by_hostname(&state.db, &event.origin)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::Unauthorized)?;

    if peer.status != 1i16 {
        return Err(ApiError::Unauthorized);
    }

    // 3. Verify signature
    let payload_str = event.payload.to_string();
    let signed_msg = format!("{}:{}:{}:{}", event.origin, event.timestamp, event.event_type, payload_str);
    if !ServerIdentity::verify(&peer.public_key, signed_msg.as_bytes(), &event.signature) {
        tracing::warn!(origin = %event.origin, "rejecting federated event: invalid signature");
        return Err(ApiError::Unauthorized);
    }

    // 4. Update peer last_seen
    let _ = mesh_peer_repo::update_last_seen(&state.db, peer.id.into()).await;

    // 5. Dispatch event
    match event.event_type.as_str() {
        "MessageCreate" => handle_federated_message(&state, &event).await?,
        "FederatedDMCreate" => handle_federated_dm(&state, &event).await?,
        other => {
            tracing::debug!(event_type = %other, origin = %event.origin, "unhandled federated event type");
        }
    }

    Ok(StatusCode::ACCEPTED)
}

/// Deliver an incoming cross-server DM to the local recipient's inbox.
///
/// Finds (or creates) the federated DM channel between the remote sender and
/// local recipient, inserts the message, then broadcasts a DmMessageCreate WS event.
async fn handle_federated_dm(state: &AppState, event: &FederatedEvent) -> Result<(), ApiError> {
    let dm: FederatedDMPayload = serde_json::from_value(event.payload.clone())
        .map_err(|_| ApiError::BadRequest("invalid FederatedDMCreate payload".into()))?;

    // Find the local recipient by username
    let recipient = user_repo::get_by_username(&state.db, &dm.recipient_username)
        .await
        .map_err(ApiError::Database)?;

    let Some(recipient) = recipient else {
        tracing::warn!(
            recipient = %dm.recipient_username,
            sender = %dm.sender_address,
            "federated DM recipient not found locally"
        );
        return Ok(()); // Not an error — peer may have stale routing info
    };

    // Get or create federated DM channel between local recipient and remote sender
    let mut sf_gen = SnowflakeGenerator::new(1, 3);
    let new_dm_id = sf_gen.next_id();
    let recipient_sf = opencorde_core::snowflake::Snowflake::new(recipient.id);

    let dm_channel_id = dm_federated_repo::get_or_create_federated_dm(
        &state.db,
        new_dm_id,
        recipient_sf,
        &dm.sender_address,
        &event.origin,
    )
    .await
    .map_err(ApiError::Database)?;

    // Insert message with federated author attribution
    let message_sf = dm.message_id.parse::<i64>()
        .map(opencorde_core::snowflake::Snowflake::new)
        .unwrap_or_else(|_| sf_gen.next_id());

    let msg = dm_federated_repo::insert_federated_dm_message(
        &state.db,
        message_sf,
        opencorde_core::snowflake::Snowflake::new(dm_channel_id),
        &dm.sender_address,
        &dm.content,
    )
    .await
    .map_err(ApiError::Database)?;

    tracing::info!(
        dm_channel = dm_channel_id,
        recipient = %dm.recipient_username,
        sender = %dm.sender_address,
        "federated DM delivered"
    );

    // Broadcast to gateway so the recipient sees it in real-time
    let ws_event = serde_json::json!({
        "type": "DmMessageCreate",
        "data": {
            "message": {
                "id": msg.id.to_string(),
                "dm_id": msg.dm_id.to_string(),
                "author_id": "0",
                "author_username": msg.author_username,
                "content": msg.content,
                "attachments": msg.attachments,
                "created_at": msg.created_at
            }
        }
    });
    let _ = state.event_tx.send(ws_event);

    Ok(())
}

/// Insert a federated message into the local DB and broadcast to WebSocket clients.
async fn handle_federated_message(state: &AppState, event: &FederatedEvent) -> Result<(), ApiError> {
    let msg: FederatedMessage = serde_json::from_value(event.payload.clone())
        .map_err(|_| ApiError::BadRequest("invalid MessageCreate payload".into()))?;

    // Find the local channel that maps to this federated channel
    // Convention: channel topic contains "federated:{origin_server}:{origin_channel_id}"
    let local_channel = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM channels WHERE topic LIKE $1 LIMIT 1"
    )
    .bind(format!("%federated:{}:{}%", event.origin, msg.channel_id))
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let Some((local_channel_id,)) = local_channel else {
        tracing::debug!(
            origin = %event.origin,
            channel = %msg.channel_id,
            "no local channel mapped for federated message — ignoring"
        );
        return Ok(());
    };

    // Insert with a synthetic author display (username@server)
    let display_name = format!("{}@{}", msg.author_username, msg.author_server);
    let federated_content = format!("[{}] {}", display_name, msg.content);

    // Use the system user (id=0 sentinel) or create a ghost user — for now use channel server_id's owner
    // Simple approach: store as a special system message with content attribution
    sqlx::query(
        "INSERT INTO messages (id, channel_id, author_id, content, created_at, updated_at) \
         SELECT $1, $2, s.owner_id, $3, NOW(), NOW() \
         FROM channels c JOIN servers s ON c.server_id = s.id WHERE c.id = $2 LIMIT 1"
    )
    .bind(msg.message_id.parse::<i64>().unwrap_or(0))
    .bind(local_channel_id)
    .bind(&federated_content)
    .execute(&state.db)
    .await
    .map_err(ApiError::Database)?;

    tracing::info!(
        origin = %event.origin,
        local_channel = local_channel_id,
        author = %display_name,
        "federated message inserted"
    );

    Ok(())
}
