//! # Repository: Mesh Peers
//! CRUD operations for peer server registry in federation.
//!
//! Manages the mesh_peers table: stores peer server hostnames, public keys,
//! connection status, and heartbeat timestamps.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake
//! - sqlx — Database access

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Row type for reading mesh peers from the database.
///
/// Maps directly to the mesh_peers table schema.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MeshPeerRow {
    /// Snowflake ID (primary key)
    pub id: i64,
    /// Peer server hostname (e.g., "mesh.example.com")
    pub hostname: String,
    /// Peer server's Ed25519 public key (hex-encoded, 64 chars)
    pub public_key: String,
    /// Connection status: 0=pending, 1=active, 2=suspended
    pub status: i16,
    /// Last successful heartbeat timestamp
    pub last_seen_at: Option<DateTime<Utc>>,
    /// Peer registration timestamp
    pub created_at: DateTime<Utc>,
}

/// Register a new mesh peer.
///
/// Creates a pending peer entry. Status defaults to 0 (pending).
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Snowflake ID for the peer
/// * `hostname` - Peer server hostname
/// * `public_key` - Peer server's Ed25519 public key (hex-encoded, 64 chars)
///
/// # Errors
/// Returns sqlx::Error if insert fails (e.g., duplicate hostname/public_key).
///
/// # Tracing
/// Logs peer registration at INFO level.
#[tracing::instrument(skip(pool))]
pub async fn register_peer(
    pool: &PgPool,
    id: Snowflake,
    hostname: &str,
    public_key: &str,
) -> Result<MeshPeerRow, sqlx::Error> {
    tracing::info!(
        hostname = %hostname,
        public_key = %public_key,
        "registering mesh peer"
    );

    let row = sqlx::query_as::<_, MeshPeerRow>(
        "INSERT INTO mesh_peers (id, hostname, public_key, status) \
         VALUES ($1, $2, $3, 0) RETURNING *",
    )
    .bind(id.as_i64())
    .bind(hostname)
    .bind(public_key)
    .fetch_one(pool)
    .await?;

    tracing::info!(peer_id = row.id, "mesh peer registered successfully");
    Ok(row)
}

/// Fetch a peer by hostname.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
///
/// # Tracing
/// Logs lookup at DEBUG level.
#[tracing::instrument(skip(pool))]
pub async fn get_by_hostname(
    pool: &PgPool,
    hostname: &str,
) -> Result<Option<MeshPeerRow>, sqlx::Error> {
    tracing::debug!(hostname = %hostname, "fetching mesh peer by hostname");

    sqlx::query_as::<_, MeshPeerRow>("SELECT * FROM mesh_peers WHERE hostname = $1")
        .bind(hostname)
        .fetch_optional(pool)
        .await
}

/// Fetch a peer by public key.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
///
/// # Tracing
/// Logs lookup at DEBUG level.
#[tracing::instrument(skip(pool))]
pub async fn get_by_public_key(
    pool: &PgPool,
    public_key: &str,
) -> Result<Option<MeshPeerRow>, sqlx::Error> {
    tracing::debug!(
        public_key = %public_key,
        "fetching mesh peer by public key"
    );

    sqlx::query_as::<_, MeshPeerRow>("SELECT * FROM mesh_peers WHERE public_key = $1")
        .bind(public_key)
        .fetch_optional(pool)
        .await
}

/// List all active peers (status = 1).
///
/// # Errors
/// Returns sqlx::Error if the query fails.
///
/// # Tracing
/// Logs count at INFO level.
#[tracing::instrument(skip(pool))]
pub async fn list_active(pool: &PgPool) -> Result<Vec<MeshPeerRow>, sqlx::Error> {
    tracing::debug!("fetching active mesh peers");

    let peers = sqlx::query_as::<_, MeshPeerRow>(
        "SELECT * FROM mesh_peers WHERE status = 1 ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    tracing::info!(count = peers.len(), "active mesh peers fetched");
    Ok(peers)
}

/// List all peers regardless of status.
///
/// Ordered by creation time, newest first.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
///
/// # Tracing
/// Logs count at INFO level.
#[tracing::instrument(skip(pool))]
pub async fn list_all(pool: &PgPool) -> Result<Vec<MeshPeerRow>, sqlx::Error> {
    tracing::debug!("fetching all mesh peers");

    let peers =
        sqlx::query_as::<_, MeshPeerRow>("SELECT * FROM mesh_peers ORDER BY created_at DESC")
            .fetch_all(pool)
            .await?;

    tracing::info!(count = peers.len(), "all mesh peers fetched");
    Ok(peers)
}

/// Update a peer's status.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Peer Snowflake ID
/// * `status` - New status (0=pending, 1=active, 2=suspended)
///
/// # Errors
/// Returns sqlx::Error if the query fails.
///
/// # Tracing
/// Logs update at INFO level with old and new status.
#[tracing::instrument(skip(pool))]
pub async fn update_status(
    pool: &PgPool,
    id: Snowflake,
    status: i16,
) -> Result<Option<MeshPeerRow>, sqlx::Error> {
    tracing::info!(
        peer_id = id.as_i64(),
        new_status = status,
        "updating mesh peer status"
    );

    let row = sqlx::query_as::<_, MeshPeerRow>(
        "UPDATE mesh_peers SET status = $1 WHERE id = $2 RETURNING *",
    )
    .bind(status)
    .bind(id.as_i64())
    .fetch_optional(pool)
    .await?;

    if row.is_some() {
        tracing::info!(peer_id = id.as_i64(), "mesh peer status updated");
    }

    Ok(row)
}

/// Update a peer's last_seen_at timestamp.
///
/// Sets last_seen_at to the current time.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
///
/// # Tracing
/// Logs heartbeat reception at DEBUG level.
#[tracing::instrument(skip(pool))]
pub async fn update_last_seen(
    pool: &PgPool,
    id: Snowflake,
) -> Result<Option<MeshPeerRow>, sqlx::Error> {
    tracing::debug!(peer_id = id.as_i64(), "updating mesh peer last_seen_at");

    let row = sqlx::query_as::<_, MeshPeerRow>(
        "UPDATE mesh_peers SET last_seen_at = NOW() WHERE id = $1 RETURNING *",
    )
    .bind(id.as_i64())
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Delete a peer from the registry.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
///
/// # Tracing
/// Logs deletion at INFO level.
#[tracing::instrument(skip(pool))]
pub async fn delete_peer(pool: &PgPool, id: Snowflake) -> Result<bool, sqlx::Error> {
    tracing::info!(peer_id = id.as_i64(), "deleting mesh peer");

    let result = sqlx::query("DELETE FROM mesh_peers WHERE id = $1")
        .bind(id.as_i64())
        .execute(pool)
        .await?;

    let deleted = result.rows_affected() > 0;
    if deleted {
        tracing::info!(peer_id = id.as_i64(), "mesh peer deleted");
    } else {
        tracing::warn!(peer_id = id.as_i64(), "mesh peer not found for deletion");
    }

    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_peer_row_struct() {
        let peer = MeshPeerRow {
            id: 123,
            hostname: "example.com".to_string(),
            public_key: "abcd1234".repeat(8),
            status: 1,
            last_seen_at: None,
            created_at: Utc::now(),
        };

        assert_eq!(peer.id, 123);
        assert_eq!(peer.hostname, "example.com");
        assert_eq!(peer.status, 1);

        let cloned = peer.clone();
        assert_eq!(cloned.id, peer.id);
        assert_eq!(cloned.hostname, peer.hostname);

        let debug_str = format!("{:?}", peer);
        assert!(debug_str.contains("123"));
        assert!(debug_str.contains("example.com"));
    }
}
