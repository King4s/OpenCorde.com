//! # Federation Client
//! Outbound server-to-server communication: registry sync, peer introduction, event forwarding.
//!
//! ## Registry Sync
//! On startup (and every 24h), fetches the public server registry from GitHub:
//! `https://raw.githubusercontent.com/King4s/opencorde-servers/master/servers.json`
//! Any new servers are added to mesh_peers as pending and introduced to.
//!
//! ## Peer Introduction
//! After adding a peer from the registry, we POST to their /federation/introduce
//! with our signed identity. If they accept, we mark them active.
//!
//! ## Event Forwarding
//! Call `forward_event()` after creating a message in a federated channel.
//! It signs the event and POSTs to all active peers.
//!
//! ## Depends On
//! - reqwest (HTTP client)
//! - ed25519-dalek (signing via ServerIdentity)
//! - opencorde-db (mesh_peer_repo)
//! - tokio (async, spawn, interval)

use std::{sync::Arc, time::Duration};

use opencorde_db::repos::mesh_peer_repo;
use reqwest::Client;
use sqlx::PgPool;

use crate::{
    config::Config,
    identity::ServerIdentity,
    routes::federation::FederatedEvent,
};

const REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/King4s/opencorde-servers/master/servers.json";
const SYNC_INTERVAL_SECS: u64 = 86_400; // 24 hours

/// A server entry from the public GitHub registry.
#[derive(serde::Deserialize)]
struct RegistryEntry {
    hostname: String,
    #[allow(dead_code)]
    pubkey: Option<String>,
    #[allow(dead_code)]
    name: Option<String>,
}

/// Spawn a background task that syncs the peer registry every 24h.
pub fn spawn_registry_sync(db: PgPool, config: Arc<Config>, identity: Arc<ServerIdentity>) {
    tokio::spawn(async move {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("failed to build HTTP client");

        let mut interval = tokio::time::interval(Duration::from_secs(SYNC_INTERVAL_SECS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            if let Err(e) = sync_registry(&client, &db, &config, &identity).await {
                tracing::warn!(error = %e, "registry sync failed");
            }
        }
    });
}

/// Fetch the registry and introduce ourselves to any unknown servers.
async fn sync_registry(
    client: &Client,
    db: &PgPool,
    config: &Config,
    identity: &ServerIdentity,
) -> anyhow::Result<()> {
    tracing::info!(url = REGISTRY_URL, "syncing peer registry");

    let entries: Vec<RegistryEntry> = client
        .get(REGISTRY_URL)
        .send()
        .await?
        .json()
        .await?;

    tracing::info!(count = entries.len(), "registry entries fetched");

    for entry in entries {
        // Skip ourselves
        if entry.hostname == config.mesh_hostname {
            continue;
        }

        // Only introduce to servers we don't know yet
        if let Ok(None) = mesh_peer_repo::get_by_hostname(db, &entry.hostname).await {
            tracing::info!(hostname = %entry.hostname, "introducing to new registry peer");
            if let Err(e) = introduce_to_peer(client, db, config, identity, &entry.hostname).await {
                tracing::warn!(
                    hostname = %entry.hostname,
                    error = %e,
                    "introduction to registry peer failed"
                );
            }
        }
    }

    Ok(())
}

/// POST to a peer's /federation/introduce endpoint.
/// If they accept, stores them as active in mesh_peers (they will also store us).
pub async fn introduce_to_peer(
    client: &Client,
    db: &PgPool,
    config: &Config,
    identity: &ServerIdentity,
    peer_hostname: &str,
) -> anyhow::Result<()> {
    let timestamp = chrono::Utc::now().timestamp();
    let signed_msg = format!("{}:{}", config.mesh_hostname, timestamp);
    let signature = identity.sign(signed_msg.as_bytes());

    let body = serde_json::json!({
        "hostname": config.mesh_hostname,
        "public_key": identity.public_key_hex,
        "server_name": config.mesh_hostname,
        "timestamp": timestamp,
        "signature": signature,
    });

    let url = format!("https://{}/api/v1/federation/introduce", peer_hostname);
    let resp = client
        .post(&url)
        .json(&body)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("peer returned {}", resp.status());
    }

    let reply: serde_json::Value = resp.json().await?;
    let their_pubkey = reply["public_key"].as_str().unwrap_or("").to_string();

    // Upsert peer as active with their public key
    let existing = mesh_peer_repo::get_by_hostname(db, peer_hostname).await?;
    if let Some(peer) = existing {
        sqlx::query(
            "UPDATE mesh_peers SET public_key = $1, status = 1, last_seen_at = NOW() WHERE id = $2"
        )
        .bind(&their_pubkey)
        .bind(peer.id)
        .execute(db)
        .await?;
    } else {
        use opencorde_core::snowflake::SnowflakeGenerator;
        let mut sf_gen = SnowflakeGenerator::new(1, 2);
        let id = sf_gen.next_id();
        sqlx::query(
            "INSERT INTO mesh_peers (id, hostname, public_key, status, last_seen_at) \
             VALUES ($1, $2, $3, 1, NOW())"
        )
        .bind(id.as_i64())
        .bind(peer_hostname)
        .bind(&their_pubkey)
        .execute(db)
        .await?;
    }

    tracing::info!(hostname = %peer_hostname, "successfully peered");
    Ok(())
}

/// Forward a signed event to a single specific peer by hostname.
///
/// Used for targeted events like cross-server DMs where only one
/// destination server needs to receive the event.
/// Failures are logged but do not block the local operation.
pub async fn forward_event_to(
    db: &PgPool,
    identity: &ServerIdentity,
    origin: &str,
    target_hostname: &str,
    event_type: &str,
    payload: serde_json::Value,
) {
    let peer = match mesh_peer_repo::get_by_hostname(db, target_hostname).await {
        Ok(Some(p)) if p.status == 1i16 => p,
        Ok(_) => {
            tracing::warn!(
                hostname = %target_hostname,
                "skipping federated DM forward: peer not active"
            );
            return;
        }
        Err(e) => {
            tracing::warn!(error = %e, hostname = %target_hostname, "peer lookup failed");
            return;
        }
    };

    let timestamp = chrono::Utc::now().timestamp();
    let payload_str = payload.to_string();
    let signed_msg = format!("{}:{}:{}:{}", origin, timestamp, event_type, payload_str);
    let signature = identity.sign(signed_msg.as_bytes());

    let event = FederatedEvent {
        origin: origin.to_string(),
        origin_pubkey: identity.public_key_hex.clone(),
        timestamp,
        event_type: event_type.to_string(),
        payload,
        signature,
    };

    let client = match Client::builder().timeout(Duration::from_secs(8)).build() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "failed to build http client for targeted forward");
            return;
        }
    };

    let url = format!("https://{}/api/v1/federation/events", peer.hostname);
    if let Err(e) = client.post(&url).json(&event).send().await {
        tracing::warn!(
            hostname = %peer.hostname,
            error = %e,
            "failed to forward targeted event to peer"
        );
    }
}

/// Forward a signed event to all active peers.
///
/// Called after creating a message in a federated channel.
/// Failures are logged but do not block the local operation.
pub async fn forward_event(
    db: &PgPool,
    identity: &ServerIdentity,
    origin: &str,
    event_type: &str,
    payload: serde_json::Value,
) {
    let peers = match mesh_peer_repo::list_active(db).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "failed to list peers for event forwarding");
            return;
        }
    };

    if peers.is_empty() {
        return;
    }

    let timestamp = chrono::Utc::now().timestamp();
    let payload_str = payload.to_string();
    let signed_msg = format!("{}:{}:{}:{}", origin, timestamp, event_type, payload_str);
    let signature = identity.sign(signed_msg.as_bytes());

    let event = FederatedEvent {
        origin: origin.to_string(),
        origin_pubkey: identity.public_key_hex.clone(),
        timestamp,
        event_type: event_type.to_string(),
        payload,
        signature,
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .expect("http client");

    for peer in peers {
        let client = client.clone();
        let event = event.clone();
        let url = format!("https://{}/api/v1/federation/events", peer.hostname);
        tokio::spawn(async move {
            if let Err(e) = client.post(&url).json(&event).send().await {
                tracing::warn!(
                    hostname = %peer.hostname,
                    error = %e,
                    "failed to forward event to peer"
                );
            }
        });
    }
}
