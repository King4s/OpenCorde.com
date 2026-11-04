//! Mention notification dispatch — authorization and mute/suppression checks.

use chrono::Utc;
use opencorde_core::{Snowflake, permissions::Permissions};
use opencorde_db::repos::member_repo;

use crate::{error::ApiError, routes::permission_check};

struct NotificationTarget {
    user_id: i64,
    unread: bool,
}

async fn resolve_notification_targets(
    pool: &sqlx::PgPool,
    mentions: &super::validation::MentionSet,
    sender_id: opencorde_core::snowflake::Snowflake,
    channel_id: i64,
    server_id: i64,
) -> Result<Vec<NotificationTarget>, ApiError> {
    use std::collections::HashSet;

    // Bind membership and preferences to the channel's actual server.
    let channel = Snowflake::new(channel_id);
    if permission_check::resolve_channel_server(pool, channel)
        .await?
        .as_i64()
        != server_id
    {
        return Err(ApiError::Forbidden);
    }

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

    // Authorization applies to every delivery, including direct push-only mentions.
    let mut eligible = Vec::with_capacity(targets.len());
    for &user_id in &targets {
        let user = Snowflake::new(user_id);
        if member_repo::get_member(pool, user, Snowflake::new(server_id))
            .await?
            .is_none()
        {
            continue;
        }
        match permission_check::effective_channel_perms(pool, user, channel).await {
            Ok(permissions) if permissions.contains(Permissions::VIEW_CHANNEL) => {}
            Ok(_) | Err(ApiError::Forbidden) | Err(ApiError::NotFound(_)) => continue,
            Err(error) => return Err(error),
        }

        let unread = should_notify_user(
            pool,
            user_id,
            channel_id,
            server_id,
            mentions.has_everyone,
            mentions.has_here,
            !mentions.role_ids.is_empty(),
        )
        .await?;

        // Preserve direct mentions' push-only suppression fallback, but only
        // after successful authorization and preference lookups. Errors are not
        // suppression decisions and must never enable the fallback.
        if unread || mentions.user_ids.contains(&user_id) {
            eligible.push(NotificationTarget { user_id, unread });
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
    type ServerNotificationSettings = (i16, Option<chrono::DateTime<Utc>>, bool, bool, bool);
    let server_settings: Option<ServerNotificationSettings> =
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
        if let Some(mu) = mute_until
            && mu > Utc::now()
        {
            return Ok(false);
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
pub(super) struct MentionNotification<'a> {
    pub mentions: &'a super::validation::MentionSet,
    pub sender_id: Snowflake,
    pub sender_username: &'a str,
    pub content_preview: &'a str,
    pub channel_id: i64,
    pub server_id: i64,
}

pub(super) async fn dispatch_mention_notifications(
    pool: &sqlx::PgPool,
    config: &crate::config::Config,
    notification: MentionNotification<'_>,
    event_tx: &tokio::sync::broadcast::Sender<serde_json::Value>,
) {
    let MentionNotification {
        mentions, sender_id, sender_username, content_preview, channel_id, server_id,
    } = notification;
    let title = format!("Mention from {}", sender_username);
    dispatch_with_push(
        pool,
        mentions,
        sender_id,
        channel_id,
        server_id,
        event_tx,
        |user_id| crate::push_sender::send_push(pool, config, user_id, &title, content_preview),
    )
    .await;
}

// Keep the external push transport at the boundary so regressions can observe
// actual dispatch attempts without contacting a notification provider.
async fn dispatch_with_push<F, Fut>(
    pool: &sqlx::PgPool,
    mentions: &super::validation::MentionSet,
    sender_id: Snowflake,
    channel_id: i64,
    server_id: i64,
    event_tx: &tokio::sync::broadcast::Sender<serde_json::Value>,
    mut send_push: F,
) where
    F: FnMut(i64) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
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

    for target in targets {
        let user_id = target.user_id;
        if !target.unread {
            send_push(user_id).await;
            continue;
        }
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
        send_push(user_id).await;

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
}

#[cfg(test)]
mod tests;
