//! # Redis Pub/Sub Event Bridge
//!
//! Bridges the local `tokio::sync::broadcast` channel with Redis pub/sub so
//! events published on one API server instance are delivered to WebSocket
//! clients connected to *any* instance.
//!
//! ## Design
//! - All events are JSON-serialized and published to a single Redis channel
//!   (`opencorde:events` by default).
//! - Each API instance runs a background subscriber task that listens on the
//!   same Redis channel and forwards incoming messages into the local
//!   `broadcast::Sender`.
//! - WebSocket clients on every instance therefore receive events regardless
//!   of which instance generated them.
//! - The local broadcast channel is still used for in-process delivery (lower
//!   latency than round-tripping through Redis for same-instance clients).
//!
//! ## Avoiding Echo Loops
//! Events received from Redis are forwarded **only** to the local broadcast
//! channel; they are **not** re-published to Redis. This prevents infinite
//! echo loops across instances.
//!
//! ## Depends On
//! - `redis` (async pub/sub, connection manager)
//! - `tokio::sync::broadcast`
//! - `tracing`
//! - `serde_json`

use futures::StreamExt;
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Default Redis channel name for cross-instance event broadcast.
const DEFAULT_CHANNEL: &str = "opencorde:events";

/// Bridges local broadcast events with Redis pub/sub.
///
/// Create one per API process, then call [`spawn_subscriber`](Self::spawn_subscriber)
/// to start the background listener.
pub struct RedisEventBridge {
    /// Connection manager for publishing commands.
    publish_conn: ConnectionManager,
    /// Redis client for creating the dedicated pub/sub connection.
    redis_client: redis::Client,
    /// Local broadcast sender — events from Redis are injected here.
    local_tx: broadcast::Sender<serde_json::Value>,
    /// Redis channel name.
    channel: String,
}

impl RedisEventBridge {
    /// Create a new bridge.
    ///
    /// # Arguments
    /// * `redis_url` — Redis connection string (e.g. `redis://127.0.0.1:6379`)
    /// * `local_tx`  — The local broadcast channel used by WebSocket handlers
    ///
    /// # Errors
    /// Returns `redis::RedisError` if the client or connection manager cannot
    /// be established.
    pub async fn new(
        redis_url: &str,
        local_tx: broadcast::Sender<serde_json::Value>,
    ) -> redis::RedisResult<Arc<Self>> {
        let redis_client = redis::Client::open(redis_url)?;
        let publish_conn = ConnectionManager::new(redis_client.clone()).await?;

        tracing::info!(channel = DEFAULT_CHANNEL, "Redis event bridge initialized");

        Ok(Arc::new(Self {
            publish_conn,
            redis_client,
            local_tx,
            channel: DEFAULT_CHANNEL.to_string(),
        }))
    }

    /// Publish an event to Redis (fire-and-forget).
    ///
    /// Errors are logged but not propagated — Redis unavailability should not
    /// break local event delivery.
    pub fn publish(&self, event: &serde_json::Value) {
        let payload = match serde_json::to_string(event) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(error = %e, "failed to serialize event for Redis");
                return;
            }
        };

        let mut conn = self.publish_conn.clone();
        let channel = self.channel.clone();
        tokio::spawn(async move {
            let result: redis::RedisResult<()> = conn.publish(&channel, &payload).await;
            if let Err(e) = result {
                tracing::warn!(error = %e, "Redis publish failed");
            }
        });
    }

    /// Spawn a background task that subscribes to the Redis channel and
    /// forwards every message into the local broadcast channel.
    ///
    /// The task runs until the Redis connection is dropped or the process
    /// exits.  This should be called **once** during server startup.
    pub fn spawn_subscriber(self: Arc<Self>) {
        tokio::spawn(async move {
            if let Err(e) = self.run_subscriber().await {
                tracing::error!(error = %e, "Redis subscriber task exited with error");
            }
        });
    }

    async fn run_subscriber(&self) -> redis::RedisResult<()> {
        let mut pubsub = self.redis_client.get_async_pubsub().await?;
        pubsub.subscribe(&self.channel).await?;

        tracing::info!(channel = %self.channel, "Redis subscriber connected");

        let mut stream = pubsub.on_message();

        while let Some(msg) = stream.next().await {
            let payload: String = match msg.get_payload::<String>() {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(error = %e, "failed to extract Redis message payload");
                    continue;
                }
            };

            let event: serde_json::Value = match serde_json::from_str(&payload) {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(error = %e, payload = %payload, "failed to parse Redis event JSON");
                    continue;
                }
            };

            // Forward to local broadcast.  This is a best-effort operation:
            // if no local receivers are active the event is dropped.
            if self.local_tx.send(event).is_err() {
                tracing::debug!("no local WebSocket receivers for Redis-forwarded event");
            }
        }

        tracing::warn!("Redis subscriber stream ended");
        Ok(())
    }
}
