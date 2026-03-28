//! # Repository: Federated Direct Messages
//! Database operations for DM channels that span two OpenCorde servers.
//!
//! ## How it works
//! When user A on server1 opens a DM with alice@server2.com:
//! - A `dm_channels` row is created with `remote_peer_address = "alice@server2.com"`
//! - Only user A is added to `dm_channel_members` (the remote user has no local account)
//! - Outbound messages go via `federation::forward_event()`
//! - Inbound messages from server2 have `author_id = NULL`, `federated_author = "alice@server2.com"`
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake
//! - sqlx (async database access)
//! - chrono (timestamps)

use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

use super::dm_repo::DmMessageRow;

/// Get or create a federated DM channel between a local user and a remote peer.
///
/// The remote peer is identified only by their address ("username@hostname").
/// Only the local user is added to `dm_channel_members`.
///
/// Returns the DM channel ID (existing or newly created).
#[tracing::instrument(skip(pool))]
pub async fn get_or_create_federated_dm(
    pool: &PgPool,
    dm_id: Snowflake,
    local_user: Snowflake,
    remote_peer_address: &str,
    remote_server: &str,
) -> Result<i64, sqlx::Error> {
    let local_user_id = local_user.as_i64();
    let dm_id_val = dm_id.as_i64();

    // Check for existing federated DM with this remote peer for this local user
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT d.id FROM dm_channels d \
         JOIN dm_channel_members m ON m.dm_channel_id = d.id AND m.user_id = $1 \
         WHERE d.remote_peer_address = $2 \
         LIMIT 1",
    )
    .bind(local_user_id)
    .bind(remote_peer_address)
    .fetch_optional(pool)
    .await?;

    if let Some(id) = existing {
        tracing::info!(dm_id = id, remote_peer = %remote_peer_address, "found existing federated dm");
        return Ok(id);
    }

    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO dm_channels (id, remote_peer_address, remote_server) VALUES ($1, $2, $3)",
    )
    .bind(dm_id_val)
    .bind(remote_peer_address)
    .bind(remote_server)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO dm_channel_members (dm_channel_id, user_id, last_read_id) VALUES ($1, $2, 0)",
    )
    .bind(dm_id_val)
    .bind(local_user_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    tracing::info!(dm_id = dm_id_val, remote_peer = %remote_peer_address, "created federated dm");
    Ok(dm_id_val)
}

/// Insert a DM message that arrived from a remote server.
///
/// `federated_author` is the "username@hostname" of the remote sender.
/// `author_id` is NULL — the sender has no local account.
#[tracing::instrument(skip(pool, content))]
pub async fn insert_federated_dm_message(
    pool: &PgPool,
    id: Snowflake,
    dm_id: Snowflake,
    federated_author: &str,
    content: &str,
) -> Result<DmMessageRow, sqlx::Error> {
    tracing::info!(
        dm_id = dm_id.as_i64(),
        federated_author = %federated_author,
        "inserting federated dm message"
    );

    let row = sqlx::query_as::<_, DmMessageRow>(
        "WITH inserted AS ( \
             INSERT INTO dm_messages (id, dm_id, author_id, federated_author, content) \
             VALUES ($1, $2, NULL, $3, $4) RETURNING * \
         ) \
         SELECT i.id, i.dm_id, COALESCE(i.author_id, 0) as author_id, \
                COALESCE(i.federated_author, '[remote]') as author_username, \
                i.content, i.attachments, i.edited_at, i.created_at \
         FROM inserted i",
    )
    .bind(id.as_i64())
    .bind(dm_id.as_i64())
    .bind(federated_author)
    .bind(content)
    .fetch_one(pool)
    .await?;

    tracing::info!(message_id = row.id, "federated dm message inserted");
    Ok(row)
}

/// Get the remote_server hostname for a DM channel, if it's federated.
///
/// Returns None for local (same-server) DM channels.
#[tracing::instrument(skip(pool))]
pub async fn get_remote_server(
    pool: &PgPool,
    dm_id: Snowflake,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar::<_, Option<String>>(
        "SELECT remote_server FROM dm_channels WHERE id = $1",
    )
    .bind(dm_id.as_i64())
    .fetch_optional(pool)
    .await
    .map(|opt| opt.flatten())
}

/// Get the remote_peer_address for a DM channel.
///
/// Returns the "username@hostname" of the remote participant for federated DMs,
/// or None for local DMs.
#[tracing::instrument(skip(pool))]
pub async fn get_remote_peer_address(
    pool: &PgPool,
    dm_id: Snowflake,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar::<_, Option<String>>(
        "SELECT remote_peer_address FROM dm_channels WHERE id = $1",
    )
    .bind(dm_id.as_i64())
    .fetch_optional(pool)
    .await
    .map(|opt| opt.flatten())
}

/// Row for federated DM channel lookup (finding DM by remote peer address).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FederatedDmRow {
    pub id: i64,
    pub remote_peer_address: String,
    pub remote_server: String,
}

/// Find a federated DM channel by local recipient and remote sender address.
///
/// Used when receiving a FederatedDMCreate event to find the local DM inbox.
#[tracing::instrument(skip(pool))]
pub async fn find_by_remote_peer(
    pool: &PgPool,
    local_user_id: i64,
    remote_peer_address: &str,
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT d.id FROM dm_channels d \
         JOIN dm_channel_members m ON m.dm_channel_id = d.id AND m.user_id = $1 \
         WHERE d.remote_peer_address = $2 \
         LIMIT 1",
    )
    .bind(local_user_id)
    .bind(remote_peer_address)
    .fetch_optional(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_federated_dm_constants() {
        // address format: username@hostname — no spaces, contains exactly one @
        let addr = "alice@chat.example.com";
        let parts: Vec<&str> = addr.splitn(2, '@').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "alice");
        assert_eq!(parts[1], "chat.example.com");
    }
}
