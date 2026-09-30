//! Send message — POST handler with automod, slowmode, mention gates, and notifications.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::Utc;
use opencorde_core::permissions::Permissions;
use opencorde_core::snowflake::{Snowflake, SnowflakeGenerator};
use opencorde_db::repos::message_repo;
use tracing::instrument;

use crate::{
    AppState, automod, error::ApiError, middleware::auth::AuthUser, routes::permission_check,
};

use super::helpers::message_row_to_response;
use super::notifications::dispatch_mention_notifications;
use super::types::{MessageResponse, SendMessageRequest};
use super::validation::{extract_mentions, parse_snowflake_id, validate_content};

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
