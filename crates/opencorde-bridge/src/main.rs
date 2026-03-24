//! # OpenCorde — Discord Bridge
//! Bidirectional message bridge connecting Discord guilds to OpenCorde servers.
//!
//! ## Architecture
//! Two concurrent tasks run in parallel:
//! 1. **Gateway loop** — Discord WebSocket → receive MESSAGE_CREATE → insert into OpenCorde DB
//! 2. **Poll loop**    — Query new OpenCorde messages → POST to Discord via webhook
//!
//! Channel mappings are stored in `bridge_channel_mappings`.
//! Discord users are mirrored as ghost users via `bridge_ghost_users`.
//!
//! ## Setup
//! 1. Create a Discord bot, enable MESSAGE_CONTENT privileged intent
//! 2. Invite bot to your guild
//! 3. Create a Discord webhook in each bridged channel
//! 4. Insert a row into bridge_channel_mappings with the channel + webhook IDs
//! 5. Set DISCORD_TOKEN + DATABASE_URL and run: cargo run --bin opencorde-bridge
//!
//! ## Depends On
//! - tokio (async runtime)
//! - tracing + tracing-subscriber (structured logging)
//! - opencorde-db (connection pool + migrations)
//! - crate::config (env config)
//! - crate::discord (bridge implementation)

mod config;
mod discord;

use std::{sync::Arc, time::Duration};

use discord::{mapper, DiscordApi};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "opencorde_bridge=info,warn".parse().unwrap()),
        )
        .init();

    let config = config::BridgeConfig::from_env()?;

    tracing::info!("connecting to database");
    let db = opencorde_db::create_pool(&config.database_url).await?;

    tracing::info!("running migrations");
    opencorde_db::run_migrations(&db).await?;

    let api = Arc::new(DiscordApi::new(&config.discord_token));
    let poll_interval = Duration::from_millis(config.poll_interval_ms);

    // Task 1: Discord → OpenCorde (gateway event loop)
    let gateway_db = db.clone();
    let gateway_token = config.discord_token.clone();
    let gateway_task = tokio::spawn(async move {
        if let Err(e) = discord::gateway::run(&gateway_token, gateway_db).await {
            tracing::error!(error = ?e, "gateway loop exited with error");
        }
    });

    // Task 2: OpenCorde → Discord (polling loop)
    let poll_db = db.clone();
    let poll_api = api.clone();
    let poll_task = tokio::spawn(async move {
        run_poll_loop(poll_db, poll_api, poll_interval).await;
    });

    tracing::info!("bridge running — gateway + poll loop active");

    tokio::select! {
        _ = gateway_task => tracing::warn!("gateway task exited"),
        _ = poll_task    => tracing::warn!("poll task exited"),
    }

    Ok(())
}

/// OpenCorde → Discord polling loop.
///
/// Every `interval`, loads all active mappings with a webhook configured,
/// fetches new messages from OpenCorde users (not ghost users), and
/// forwards them to Discord via the channel's webhook.
async fn run_poll_loop(
    db: sqlx::PgPool,
    api: Arc<DiscordApi>,
    interval: Duration,
) {
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;

        let mappings = match mapper::load_active_mappings(&db).await {
            Ok(m) => m,
            Err(e) => {
                tracing::error!(error = ?e, "failed to load channel mappings");
                continue;
            }
        };

        for mapping in &mappings {
            // Skip channels without a webhook configured (no Discord→OpenCorde direction)
            let (Some(webhook_id), Some(webhook_token)) =
                (mapping.discord_webhook_id, &mapping.discord_webhook_token)
            else {
                continue;
            };

            let pending = match mapper::pending_opencorde_messages(&db, mapping).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(
                        error = ?e,
                        opencorde_channel_id = mapping.opencorde_channel_id,
                        "failed to fetch pending messages"
                    );
                    continue;
                }
            };

            for msg in &pending {
                match api
                    .send_webhook_message(
                        webhook_id as u64,
                        webhook_token,
                        &msg.author_username,
                        &msg.content,
                        msg.author_avatar_url.as_deref(),
                    )
                    .await
                {
                    Ok(()) => {
                        tracing::info!(
                            opencorde_channel_id = mapping.opencorde_channel_id,
                            author = %msg.author_username,
                            "OpenCorde → Discord message forwarded"
                        );
                        let _ = mapper::update_opencorde_cursor(
                            &db,
                            mapping.id,
                            msg.opencorde_msg_id,
                        )
                        .await;
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = ?e,
                            webhook_id,
                            "failed to forward message to Discord"
                        );
                    }
                }
            }
        }
    }
}
