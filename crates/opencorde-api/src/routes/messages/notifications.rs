//! Mention notification dispatch — target resolution, mute/suppression checks.

use chrono::Utc;

use crate::error::ApiError;

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
pub(super) async fn dispatch_mention_notifications(
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
