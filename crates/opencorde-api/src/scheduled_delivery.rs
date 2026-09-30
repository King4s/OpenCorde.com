//! # Scheduled Message Delivery
//! Background task that periodically checks for due scheduled messages
//! and delivers them to their target channels.
//!
//! Runs every 15 seconds. For each due message:
//! 1. Verify the channel still exists
//! 2. Verify the author still has SEND_MESSAGES permission
//! 3. Generate a new Snowflake message ID
//! 4. Create the message in the messages table
//! 5. Broadcast MessageCreate via WebSocket
//! 6. Handle search indexing
//! 7. Mark the scheduled entry as delivered
//!
//! Edge cases:
//! - Deleted channel → skip delivery (cancelled implicitly via ON DELETE CASCADE)
//! - Permission loss → mark as delivered but log warning (message is rejected)

use std::sync::Arc;

use chrono::Utc;
use opencorde_core::permissions::Permissions;
use opencorde_core::snowflake::{Snowflake, SnowflakeGenerator};
use opencorde_db::repos::{message_repo, scheduled_message_repo};
use sqlx::PgPool;
use tokio::sync::broadcast;

use crate::error::ApiError;
use crate::routes::permission_check;

/// Spawn the scheduled message delivery background task.
///
/// Runs every 15 seconds, fetching up to 20 due messages per tick.
/// Non-blocking — errors on individual messages are logged and skipped.
pub fn spawn_delivery_worker(
    db: PgPool,
    event_tx: Arc<broadcast::Sender<serde_json::Value>>,
    search: Option<Arc<opencorde_search::SearchEngine>>,
) {
    tokio::spawn(async move {
        tracing::info!("scheduled message delivery worker started (interval: 15s)");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));

        loop {
            interval.tick().await;
            deliver_due_messages(&db, &event_tx, &search).await;
        }
    });
}

/// Fetch due messages and attempt delivery for each.
async fn deliver_due_messages(
    db: &PgPool,
    event_tx: &broadcast::Sender<serde_json::Value>,
    search: &Option<Arc<opencorde_search::SearchEngine>>,
) {
    let due = match scheduled_message_repo::fetch_due_messages(db, 20).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "failed to fetch due scheduled messages");
            return;
        }
    };

    if due.is_empty() {
        return;
    }

    tracing::info!(count = due.len(), "delivering due scheduled messages");

    for row in due {
        if let Err(e) = deliver_one(db, event_tx, search, row).await {
            tracing::warn!(
                error = %e,
                "failed to deliver scheduled message"
            );
        }
    }
}

/// Attempt to deliver a single scheduled message.
async fn deliver_one(
    db: &PgPool,
    event_tx: &broadcast::Sender<serde_json::Value>,
    search: &Option<Arc<opencorde_search::SearchEngine>>,
    scheduled: scheduled_message_repo::ScheduledMessageRow,
) -> Result<(), anyhow::Error> {
    let author_id = Snowflake::new(scheduled.author_id);
    let channel_id = Snowflake::new(scheduled.channel_id);
    let scheduled_id = Snowflake::new(scheduled.id);

    tracing::info!(
        scheduled_id = scheduled.id,
        channel_id = scheduled.channel_id,
        author_id = scheduled.author_id,
        "delivering scheduled message"
    );

    // Check if channel still exists and user still has permission
    match permission_check::require_channel_perm(
        db,
        author_id,
        channel_id,
        Permissions::SEND_MESSAGES,
    )
    .await
    {
        Ok(()) => {} // Permission OK, continue
        Err(ApiError::NotFound(_)) => {
            // Channel deleted — mark as delivered (cancelled effectively)
            tracing::warn!(
                scheduled_id = scheduled.id,
                "channel not found, skipping delivery"
            );
            let _ = scheduled_message_repo::mark_delivered(db, scheduled_id).await;
            return Ok(());
        }
        Err(ApiError::Forbidden) => {
            // Permission lost — mark as delivered with warning
            tracing::warn!(
                scheduled_id = scheduled.id,
                author_id = scheduled.author_id,
                "author lost send permission, skipping delivery"
            );
            let _ = scheduled_message_repo::mark_delivered(db, scheduled_id).await;
            return Ok(());
        }
        Err(e) => {
            // Database error or other — don't mark delivered, will retry
            return Err(e.into());
        }
    }

    // Fetch server_id from channel (needed for search indexing)
    let server_id: i64 = sqlx::query_scalar("SELECT server_id FROM channels WHERE id = $1")
        .bind(channel_id.as_i64())
        .fetch_optional(db)
        .await?
        .unwrap_or(0);

    // Generate a new message ID for delivery time
    let mut generator = SnowflakeGenerator::new(3, 0);
    let message_id = generator.next_id();

    // Create the message in the messages table
    let reply_to_id = scheduled.reply_to_id.map(Snowflake::new);
    let row = message_repo::create_message(
        db,
        message_id,
        channel_id,
        author_id,
        &scheduled.content,
        reply_to_id,
        scheduled.attachments.clone(),
        None, // no thread_id for scheduled messages
        None, // no forwarded_from_id for scheduled messages
    )
    .await?;

    let response = crate::routes::messages::message_row_to_response(row);

    // Broadcast MessageCreate event
    let event = serde_json::json!({
        "type": "MessageCreate",
        "data": { "message": response }
    });
    if event_tx.send(event).is_err() {
        tracing::debug!("no WebSocket subscribers for scheduled MessageCreate event");
    }

    // Index in full-text search (non-blocking)
    if let Some(engine) = search {
        let engine = engine.clone();
        let msg_id = message_id.as_i64() as u64;
        let ch_id = channel_id.as_i64() as u64;
        let srv_id = server_id as u64;
        let auth_id = author_id.as_i64() as u64;
        let text = scheduled.content.clone();
        let ts = Utc::now().timestamp() as u64;
        tokio::spawn(async move {
            if let Ok(mut indexer) = engine.make_indexer(50_000_000) {
                let _ = indexer
                    .index_message(msg_id, ch_id, srv_id, auth_id, &text, ts)
                    .and_then(|_| indexer.commit());
            }
        });
    }

    // Mark as delivered
    scheduled_message_repo::mark_delivered(db, scheduled_id).await?;

    tracing::info!(
        scheduled_id = scheduled.id,
        message_id = message_id.as_i64(),
        "scheduled message delivered successfully"
    );

    Ok(())
}
