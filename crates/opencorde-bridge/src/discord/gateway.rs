//! # Discord Gateway Event Loop
//! Connects to Discord's WebSocket gateway and processes incoming events.
//!
//! ## Responsibilities
//! - Maintain a single-shard gateway connection (sufficient for bridge bots)
//! - Handle MESSAGE_CREATE: look up channel mapping, create ghost user, insert message
//! - Reconnect on transient errors (twilight-gateway handles session resumption)
//!
//! ## Intents Required
//! - GUILD_MESSAGES   — receive messages in guilds
//! - MESSAGE_CONTENT  — privileged intent, must be enabled in Discord Developer Portal
//!
//! ## Depends On
//! - twilight-gateway 0.15 (next_event() → Result<Event, E>, no filter arg)
//! - twilight-model (Event, Message, User types)
//! - sqlx (database)

use sqlx::PgPool;
use twilight_gateway::{Intents, Shard, ShardId};
use twilight_model::gateway::{event::Event, payload::incoming::MessageCreate};

use super::{mapper, puppet};

/// Run the Discord gateway event loop.
///
/// Connects to Discord with a single shard and processes events indefinitely.
/// Returns on unrecoverable error.
#[tracing::instrument(skip(token, db))]
pub async fn run(token: &str, db: PgPool) -> anyhow::Result<()> {
    // MESSAGE_CONTENT is a privileged intent — enable in Discord Developer Portal.
    let intents = Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT;

    let mut shard = Shard::new(ShardId::ONE, token.to_string(), intents);

    tracing::info!("connecting to Discord gateway");

    loop {
        // In twilight-gateway 0.15, next_event() takes no filter args.
        // It returns Result<Event, ReceiveMessageError> — never None.
        match shard.next_event().await {
            Ok(event) => {
                if let Err(e) = handle_event(event, &db).await {
                    tracing::warn!(error = ?e, "error handling gateway event");
                }
            }
            Err(e) => {
                // Log the error; twilight-gateway handles reconnection internally.
                // Sleep briefly to prevent a tight loop on persistent failure.
                tracing::error!(error = ?e, "gateway receive error — shard will reconnect");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
}

/// Dispatch an incoming gateway event.
async fn handle_event(event: Event, db: &PgPool) -> anyhow::Result<()> {
    match event {
        Event::Ready(ready) => {
            tracing::info!(
                username = %ready.user.name,
                guilds = ready.guilds.len(),
                "Discord gateway ready"
            );
        }
        Event::GuildCreate(_) => {
            tracing::debug!("guild_create received");
        }
        Event::MessageCreate(msg) => {
            handle_message_create(*msg, db).await?;
        }
        _ => {}
    }
    Ok(())
}

/// Handle a Discord MESSAGE_CREATE event.
///
/// 1. Skip bot/webhook messages (prevents bridge loops)
/// 2. Look up the channel mapping
/// 3. Find or create the OpenCorde ghost user for the Discord author
/// 4. Insert the message into OpenCorde and advance the Discord cursor
#[tracing::instrument(skip(msg, db), fields(
    discord_channel_id = %msg.0.channel_id,
    discord_author = %msg.0.author.name,
))]
async fn handle_message_create(msg: MessageCreate, db: &PgPool) -> anyhow::Result<()> {
    let message = &msg.0;

    // Skip bots and empty content to prevent bridge loops
    if message.author.bot || message.content.is_empty() {
        return Ok(());
    }

    let discord_channel_id: u64 = message.channel_id.get();

    let Some(mapping) = mapper::get_by_discord_channel(db, discord_channel_id).await? else {
        return Ok(());
    };

    // Build CDN avatar URL — ImageHash implements Display as the raw hash string
    let avatar_url = message.author.avatar.as_ref().map(|hash| {
        format!(
            "https://cdn.discordapp.com/avatars/{}/{}.webp?size=128",
            message.author.id.get(),
            hash
        )
    });

    let author_id = puppet::find_or_create(
        db,
        message.author.id.get(),
        &message.author.name,
        avatar_url.as_deref(),
    )
    .await?;

    mapper::insert_discord_message(
        db,
        mapping.opencorde_channel_id,
        author_id,
        &message.content,
    )
    .await?;

    #[allow(clippy::cast_possible_wrap)]
    let discord_msg_id = message.id.get() as i64;
    mapper::update_discord_cursor(db, mapping.id, discord_msg_id).await?;

    tracing::info!(
        discord_channel_id,
        opencorde_channel_id = mapping.opencorde_channel_id,
        author = %message.author.name,
        "Discord → OpenCorde message bridged"
    );

    Ok(())
}
