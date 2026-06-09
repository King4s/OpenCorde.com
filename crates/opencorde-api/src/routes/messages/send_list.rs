//! # Message Send & List Handlers
//! HTTP handlers for sending and listing messages.
//!
//! ## Endpoints
//! - POST /api/v1/channels/{channel_id}/messages — Send message
//! - GET /api/v1/channels/{channel_id}/messages — List messages (cursor pagination)
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db::repos::message_repo (database operations)
//! - opencorde_core::Snowflake (ID generation)
//! - crate::middleware::auth::AuthUser (authentication)
//! - crate::AppState (application state)

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::Utc;
use opencorde_core::snowflake::{Snowflake, SnowflakeGenerator};
use opencorde_db::repos::message_repo;
use tracing::instrument;

use opencorde_core::permissions::Permissions;

use crate::{
    AppState, automod, error::ApiError, middleware::auth::AuthUser, routes::permission_check,
};

use super::types::{MessageQuery, MessageResponse, ReplyContextResponse, SendMessageRequest};
use super::validation::{extract_mentions, parse_snowflake_id, validate_content, validate_limit};

/// Convert MessageRow to MessageResponse.
pub fn message_row_to_response(row: message_repo::MessageRow) -> MessageResponse {
    let reply_to = match (
        row.reply_to_id,
        row.reply_author_username,
        row.reply_content_preview,
    ) {
        (Some(id), Some(author), Some(content)) => Some(ReplyContextResponse {
            id: id.to_string(),
            author_username: author,
            content,
        }),
        _ => None,
    };
    MessageResponse {
        id: row.id.to_string(),
        channel_id: row.channel_id.to_string(),
        author_id: row.author_id.to_string(),
        author_username: row.author_username,
        content: row.content,
        attachments: row.attachments,
        edited_at: row.edited_at,
        created_at: row.created_at,
        reply_to_id: row.reply_to_id.map(|id| id.to_string()),
        reply_to,
        forwarded_from: None,
    }
}

/// POST /api/v1/channels/{channel_id}/messages — Send a message to a channel.
///
/// Requires authentication. Generates a new Snowflake ID for the message.
/// Content must be 1-4000 characters.
///
/// Enforces @everyone/@here permission gates, role mentionable checks,
/// and dispatches notification counts to affected users.
///
/// Returns 201 Created with the new message.
#[instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
pub async fn send_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<MessageResponse>), ApiError> {
    tracing::info!("sending message to channel");

    // Parse and validate channel ID
    let channel_id_sf = parse_snowflake_id(&channel_id)?;
    tracing::debug!(channel_id = channel_id_sf.as_i64(), "parsed channel id");

    // Verify the user can send messages in this channel
    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id_sf,
        Permissions::SEND_MESSAGES,
    )
    .await?;

    // Validate content
    validate_content(&req.content)?;
    tracing::debug!(content_len = req.content.len(), "content validated");

    // Parse reply_to_id if provided
    let reply_to_id = req
        .reply_to_id
        .as_deref()
        .map(parse_snowflake_id)
        .transpose()?;
    tracing::debug!(reply_to_id = ?reply_to_id, "reply_to_id parsed");

    // Fetch server_id and slowmode_delay from channel
    let channel_info: (i64, i64, i32) =
        sqlx::query_as("SELECT id, server_id, slowmode_delay FROM channels WHERE id = $1")
            .bind(channel_id_sf.as_i64())
            .fetch_optional(&state.db)
            .await
            .map_err(ApiError::Database)?
            .ok_or(ApiError::NotFound("channel not found".to_string()))?;

    let server_id = Snowflake::new(channel_info.1);
    let slowmode_delay = channel_info.2;

    // Parse mentions from content before permission gates
    let mentions = extract_mentions(&req.content);

    // Gate: @everyone / @here requires MENTION_EVERYONE permission in the channel
    if mentions.has_everyone || mentions.has_here {
        tracing::debug!(
            has_everyone = mentions.has_everyone,
            has_here = mentions.has_here,
            "checking MENTION_EVERYONE permission"
        );
        permission_check::require_channel_perm(
            &state.db,
            auth.user_id,
            channel_id_sf,
            Permissions::MENTION_EVERYONE,
        )
        .await?;
    }

    // Gate: role mentions require each role to be mentionable
    for &role_id in &mentions.role_ids {
        let mentionable: Option<bool> =
            sqlx::query_scalar("SELECT mentionable FROM roles WHERE id = $1")
                .bind(role_id)
                .fetch_optional(&state.db)
                .await
                .map_err(ApiError::Database)?;

        match mentionable {
            Some(true) => { /* role is mentionable — allow */ }
            Some(false) => {
                tracing::warn!(role_id, "role is not mentionable");
                return Err(ApiError::Forbidden);
            }
            None => {
                return Err(ApiError::NotFound(format!("role {} not found", role_id)));
            }
        }
    }

    // Enforce server verification level (requires member tenure check)
    crate::routes::helpers::check_verification_level(&state.db, auth.user_id, server_id, true)
        .await?;

    // Enforce slowmode: check when this user last sent a message in this channel
    if slowmode_delay > 0 {
        let last_sent: Option<chrono::DateTime<Utc>> = sqlx::query_scalar(
            "SELECT created_at FROM messages \
             WHERE channel_id = $1 AND author_id = $2 \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(channel_id_sf.as_i64())
        .bind(auth.user_id.as_i64())
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::Database)?;

        if let Some(last) = last_sent {
            let elapsed = Utc::now().signed_duration_since(last).num_seconds();
            let remaining = slowmode_delay as i64 - elapsed;
            if remaining > 0 {
                tracing::info!(
                    user_id = auth.user_id.as_i64(),
                    channel_id = channel_id_sf.as_i64(),
                    retry_after = remaining,
                    "slowmode: message rejected"
                );
                return Err(ApiError::RateLimited {
                    retry_after: remaining as u64,
                });
            }
        }
    }

    // Check AutoMod rules
    match automod::check_message(&state.db, server_id, &req.content).await {
        automod::AutomodResult::Block { rule_name, .. } => {
            tracing::info!(rule = %rule_name, "message blocked by automod");
            return Err(ApiError::BadRequest(format!(
                "message blocked by automod rule: {}",
                rule_name
            )));
        }
        automod::AutomodResult::Allow => {}
    }

    // Generate message ID
    let mut generator = SnowflakeGenerator::new(3, 0);
    let message_id = generator.next_id();
    tracing::debug!(message_id = message_id.as_i64(), "generated message id");

    // Create message in database
    let attachments = req.attachments.unwrap_or_else(|| serde_json::json!([]));
    let row = message_repo::create_message(
        &state.db,
        message_id,
        channel_id_sf,
        auth.user_id,
        &req.content,
        reply_to_id,
        attachments,
        None,
        None,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to create message");
        ApiError::Database(e)
    })?;

    tracing::info!(
        message_id = row.id,
        channel_id = row.channel_id,
        "message created successfully"
    );

    let response = message_row_to_response(row);

    // Build mention metadata for the MessageCreate event
    let mention_data = serde_json::json!({
        "users": mentions.user_ids,
        "roles": mentions.role_ids,
        "everyone": mentions.has_everyone,
        "here": mentions.has_here,
    });

    // Broadcast MessageCreate event to all connected WebSocket clients.
    // Includes mention metadata so clients can decide whether to show badges.
    let event = serde_json::json!({
        "type": "MessageCreate",
        "data": {
            "message": response,
            "mentions": mention_data,
        }
    });
    if state.event_tx.send(event).is_err() {
        tracing::debug!("no WebSocket subscribers for MessageCreate event");
    }

    // Compute notification targets and dispatch mention counts + push notifications.
    if !mentions.is_empty() {
        let db = state.db.clone();
        let config = state.config.clone();
        let sender_id = auth.user_id;
        let sender_username = response.author_username.clone();
        let content_preview: String = req.content.chars().take(80).collect();
        let channel_i64 = channel_id_sf.as_i64();
        let server_i64 = server_id.as_i64();
        let message_id_i64 = message_id.as_i64();
        let event_tx = state.event_tx.clone();

        tokio::spawn(async move {
            dispatch_mention_notifications(
                &db,
                &config,
                &mentions,
                sender_id,
                &sender_username,
                &content_preview,
                channel_i64,
                server_i64,
                message_id_i64,
                &event_tx,
            )
            .await;
        });
    }

    // Index message in full-text search (non-blocking, non-fatal)
    if let Some(ref engine) = state.search {
        let engine = engine.clone();
        let msg_id = response.id.parse::<u64>().unwrap_or(0);
        let ch_id = response.channel_id.parse::<u64>().unwrap_or(0);
        let srv_id = server_id.as_i64() as u64;
        let auth_id = auth.user_id.as_i64() as u64;
        let text = req.content.clone();
        let ts = chrono::Utc::now().timestamp() as u64;
        tokio::spawn(async move {
            if let Ok(mut indexer) = engine.make_indexer(50_000_000) {
                let _ = indexer
                    .index_message(msg_id, ch_id, srv_id, auth_id, &text, ts)
                    .and_then(|_| indexer.commit());
            }
        });
    }

    Ok((StatusCode::CREATED, Json(response)))
}

/// Compute the set of user IDs that should receive a mention notification,
/// respecting per-user notification and suppression settings.
///
/// For @everyone/@here: all server members except sender.
/// For user mentions: the explicitly mentioned user IDs (minus sender).
/// For role mentions: all members holding any of the mentioned roles.
///
/// Users are excluded if they have:
/// - Channel notification level 2 (MUTED)
/// - Server notification level 2 (NOTHING) or active mute_until
/// - suppress_everyone enabled (for @everyone mentions)
/// - suppress_here enabled (for @here mentions)
/// - suppress_role_mentions enabled (for role mentions)
async fn resolve_notification_targets(
    pool: &sqlx::PgPool,
    mentions: &super::validation::MentionSet,
    sender_id: opencorde_core::snowflake::Snowflake,
    channel_id: i64,
    server_id: i64,
) -> Result<Vec<i64>, ApiError> {
    use std::collections::HashSet;

    let sender_i64 = sender_id.as_i64();
    let mut targets = HashSet::new();

    // Collect raw targets based on mention type
    if mentions.has_everyone || mentions.has_here {
        // Everyone/here: all server members except sender
        let members: Vec<(i64,)> =
            sqlx::query_as("SELECT user_id FROM server_members WHERE server_id = $1")
                .bind(server_id)
                .fetch_all(pool)
                .await
                .map_err(ApiError::Database)?;

        for (uid,) in members {
            if uid != sender_i64 {
                targets.insert(uid);
            }
        }
    }

    // User mentions
    for &uid in &mentions.user_ids {
        if uid != sender_i64 {
            targets.insert(uid);
        }
    }

    // Role mentions: members holding any of the mentioned roles
    if !mentions.role_ids.is_empty() {
        // Use a query that finds all members with these roles, excluding sender
        let role_members: Vec<(i64,)> = sqlx::query_as(
            "SELECT DISTINCT mr.user_id FROM member_roles mr \
             WHERE mr.server_id = $1 AND mr.role_id = ANY($2)",
        )
        .bind(server_id)
        .bind(&mentions.role_ids)
        .fetch_all(pool)
        .await
        .map_err(ApiError::Database)?;

        for (uid,) in role_members {
            if uid != sender_i64 {
                targets.insert(uid);
            }
        }
    }

    if targets.is_empty() {
        return Ok(Vec::new());
    }

    // Filter out muted / suppressed users
    let mut eligible = Vec::with_capacity(targets.len());
    for &user_id in &targets {
        let should_notify = should_notify_user(
            pool,
            user_id,
            channel_id,
            server_id,
            mentions.has_everyone,
            mentions.has_here,
            !mentions.role_ids.is_empty(),
        )
        .await
        .unwrap_or(false);

        if should_notify {
            eligible.push(user_id);
        }
    }

    Ok(eligible)
}

/// Check whether a user should be notified for a mention in a channel,
/// accounting for channel-level mute, server-level mute, and suppression flags.
async fn should_notify_user(
    pool: &sqlx::PgPool,
    user_id: i64,
    channel_id: i64,
    server_id: i64,
    is_everyone: bool,
    is_here: bool,
    has_role_mention: bool,
) -> Result<bool, ApiError> {
    // Check channel-level notification setting (0=all, 1=mentions-only, 2=muted)
    let channel_level: Option<i16> = sqlx::query_scalar(
        "SELECT level FROM channel_notification_settings \
         WHERE user_id = $1 AND channel_id = $2",
    )
    .bind(user_id)
    .bind(channel_id)
    .fetch_optional(pool)
    .await
    .map_err(ApiError::Database)?;

    if channel_level == Some(2) {
        // Channel muted — no notification
        return Ok(false);
    }

    // Check server-level notification settings
    let server_settings: Option<(i16, Option<chrono::DateTime<Utc>>, bool, bool, bool)> =
        sqlx::query_as(
            "SELECT level, mute_until, suppress_everyone, suppress_here, suppress_role_mentions \
             FROM server_notification_settings \
             WHERE user_id = $1 AND server_id = $2",
        )
        .bind(user_id)
        .bind(server_id)
        .fetch_optional(pool)
        .await
        .map_err(ApiError::Database)?;

    if let Some((server_level, mute_until, se, sh, sr)) = server_settings {
        // Server fully muted
        if server_level == 2 {
            return Ok(false);
        }

        // Active mute
        if let Some(mu) = mute_until {
            if mu > Utc::now() {
                return Ok(false);
            }
        }

        // Suppression flags
        if is_everyone && se {
            return Ok(false);
        }
        if is_here && sh {
            return Ok(false);
        }
        if has_role_mention && sr {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Dispatch mention notifications: increment mention_count in channel_read_state,
/// send push notifications, and broadcast ChannelUnreadUpdate events.
async fn dispatch_mention_notifications(
    pool: &sqlx::PgPool,
    config: &crate::config::Config,
    mentions: &super::validation::MentionSet,
    sender_id: opencorde_core::snowflake::Snowflake,
    sender_username: &str,
    content_preview: &str,
    channel_id: i64,
    server_id: i64,
    _message_id: i64,
    event_tx: &tokio::sync::broadcast::Sender<serde_json::Value>,
) {
    let targets = match resolve_notification_targets(
        pool, mentions, sender_id, channel_id, server_id,
    )
    .await
    {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "failed to resolve notification targets");
            return;
        }
    };

    tracing::info!(
        target_count = targets.len(),
        channel_id,
        "dispatching mention notifications"
    );

    for &user_id in &targets {
        // Increment mention_count in read state
        if let Err(e) = sqlx::query(
            "INSERT INTO channel_read_state (user_id, channel_id, last_read_id, mention_count, updated_at) \
             VALUES ($1, $2, 0, 1, NOW()) \
             ON CONFLICT (user_id, channel_id) DO UPDATE \
             SET mention_count = channel_read_state.mention_count + 1, updated_at = NOW()",
        )
        .bind(user_id)
        .bind(channel_id)
        .execute(pool)
        .await
        {
            tracing::error!(user_id, error = %e, "failed to increment mention_count");
        }

        // Send push notification
        crate::push_sender::send_push(
            pool,
            config,
            user_id,
            &format!("Mention from {}", sender_username),
            content_preview,
        )
        .await;

        // Broadcast ChannelUnreadUpdate so other sessions of this user see the badge
        let unread_event = serde_json::json!({
            "type": "ChannelUnreadUpdate",
            "data": {
                "user_id": user_id.to_string(),
                "channel_id": channel_id.to_string(),
            }
        });
        if event_tx.send(unread_event).is_err() {
            tracing::debug!("no WebSocket subscribers for ChannelUnreadUpdate event");
        }
    }

    // Also send push notifications for explicitly mentioned users that we parsed
    // (these go through even if suppressed, because direct @user mentions still
    //  trigger a push in Discord's model — it's the badge that's suppressed)
    for &uid in &mentions.user_ids {
        if uid != sender_id.as_i64() && !targets.contains(&uid) {
            crate::push_sender::send_push(
                pool,
                config,
                uid,
                &format!("Mention from {}", sender_username),
                content_preview,
            )
            .await;
        }
    }
}

/// GET /api/v1/channels/{channel_id}/messages — List messages in a channel.
///
/// Requires authentication. Supports cursor-based pagination with optional
/// `before` and `after` cursors, and adjustable `limit` (1-100, default 50).
///
/// Returns array of messages ordered newest first.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn list_messages(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
    Query(query): Query<MessageQuery>,
) -> Result<Json<Vec<MessageResponse>>, ApiError> {
    tracing::info!("listing channel messages");

    // Parse channel ID
    let channel_id_sf = parse_snowflake_id(&channel_id)?;
    tracing::debug!(channel_id = channel_id_sf.as_i64(), "parsed channel id");

    // Verify the user can view this channel
    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id_sf,
        Permissions::VIEW_CHANNEL,
    )
    .await?;

    // Parse cursor IDs if provided
    let before = query
        .before
        .as_deref()
        .map(parse_snowflake_id)
        .transpose()?;
    let after = query.after.as_deref().map(parse_snowflake_id).transpose()?;

    // Validate and set limit
    let limit = validate_limit(query.limit);
    tracing::debug!(before = ?before, after = ?after, limit = limit, "pagination validated");

    // Fetch messages from database
    let rows = message_repo::list_by_channel(&state.db, channel_id_sf, before, after, limit)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to list messages");
            ApiError::Database(e)
        })?;

    tracing::info!(count = rows.len(), "messages fetched successfully");

    let messages = rows.into_iter().map(message_row_to_response).collect();
    Ok(Json(messages))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_row_to_response() {
        use chrono::Utc;
        use serde_json::json;

        let now = Utc::now();
        let row = message_repo::MessageRow {
            id: 777888999,
            channel_id: 555666777,
            author_id: 111222333,
            content: "Test message".to_string(),
            attachments: json!([]),
            edited_at: None,
            created_at: now,
            author_username: "testuser".to_string(),
            reply_to_id: None,
            reply_author_username: None,
            reply_content_preview: None,
            thread_id: None,
            forwarded_from_id: None,
        };

        let response = message_row_to_response(row);
        assert_eq!(response.id, "777888999");
        assert_eq!(response.channel_id, "555666777");
        assert_eq!(response.author_id, "111222333");
        assert_eq!(response.author_username, "testuser");
        assert_eq!(response.content, "Test message");
        assert!(response.edited_at.is_none());
        assert!(response.reply_to_id.is_none());
        assert!(response.reply_to.is_none());
    }

    #[test]
    fn test_message_row_to_response_with_reply() {
        use chrono::Utc;
        use serde_json::json;

        let now = Utc::now();
        let row = message_repo::MessageRow {
            id: 777888999,
            channel_id: 555666777,
            author_id: 111222333,
            content: "Reply message".to_string(),
            attachments: json!([]),
            edited_at: None,
            created_at: now,
            author_username: "testuser".to_string(),
            reply_to_id: Some(123456),
            reply_author_username: Some("originaluser".to_string()),
            reply_content_preview: Some("Original content".to_string()),
            thread_id: None,
            forwarded_from_id: None,
        };

        let response = message_row_to_response(row);
        assert_eq!(response.id, "777888999");
        assert_eq!(response.reply_to_id, Some("123456".to_string()));
        assert!(response.reply_to.is_some());
        let ctx = response.reply_to.unwrap();
        assert_eq!(ctx.author_username, "originaluser");
        assert_eq!(ctx.content, "Original content");
    }
}
