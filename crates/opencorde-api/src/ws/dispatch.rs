//! Authorize event delivery against current database state, never connection snapshots.
//! Deleted channels fail closed: their ACL no longer exists, so ChannelDelete is
//! suppressed and clients must refresh their channel list to remove stale entries.

use opencorde_core::{Snowflake, permissions::Permissions};
use opencorde_db::repos::{dm_repo, member_repo};
use sqlx::PgPool;

use crate::{error::ApiError, routes::permission_check::effective_channel_perms};

#[derive(Debug, PartialEq)]
enum Audience {
    Channel(i64),
    MlsChannel(i64),
    Dm(i64),
    Server(i64),
    User(i64),
    Public,
    Deny,
}

fn audience(event: &serde_json::Value) -> Audience {
    let id = |path: &str| event.pointer(path)?.as_str()?.parse::<i64>().ok();
    let channel = |path| id(path).map(Audience::Channel).unwrap_or(Audience::Deny);
    let server = |path| id(path).map(Audience::Server).unwrap_or(Audience::Deny);
    match event.get("type").and_then(|v| v.as_str()) {
        Some("E2eeSync") => id("/data/channel_id")
            .map(Audience::MlsChannel)
            .unwrap_or(Audience::Deny),
        Some("MessageCreate" | "MessageUpdate") => channel("/data/message/channel_id"),
        Some(
            "MessageDelete" | "TypingStart" | "ReactionAdd" | "ReactionRemove" | "ChannelDelete",
        ) => channel("/data/channel_id"),
        Some("ChannelCreate" | "ChannelUpdate") => channel("/data/channel/id"),
        Some("RoleCreate" | "RoleUpdate" | "RoleDelete" | "MemberUpdate") => {
            server("/data/server_id")
        }
        Some("ServerUpdate") => server("/data/server/id"),
        Some("DmMessageCreate") => id("/data/message/dm_id")
            .map(Audience::Dm)
            .unwrap_or(Audience::Deny),
        Some("ChannelAck") => id("/data/user_id")
            .map(Audience::User)
            .unwrap_or(Audience::Deny),
        // Existing presence/member broadcast policy is outside channel delivery.
        Some("PresenceUpdate" | "MemberJoin" | "MemberLeave") => Audience::Public,
        _ => Audience::Deny,
    }
}

/// Re-check membership and VIEW_CHANNEL for every event, including metadata.
/// Missing resources and all lookup failures deny delivery.
pub async fn should_dispatch(event: &serde_json::Value, pool: &PgPool, user: Snowflake) -> bool {
    if let Audience::MlsChannel(id) = audience(event)
        && crate::routes::e2ee::sync::require_active(pool, user, Snowflake::new(id))
            .await
            .is_err()
    {
        return false;
    }
    match authorize(audience(event), pool, user).await {
        Ok(allowed) => allowed,
        Err(_) => {
            tracing::debug!(
                user_id = user.as_i64(),
                "WebSocket authorization lookup failed; event withheld"
            );
            false
        }
    }
}

async fn authorize(audience: Audience, pool: &PgPool, user: Snowflake) -> Result<bool, ApiError> {
    match audience {
        Audience::Public => Ok(true),
        Audience::Deny => Ok(false),
        Audience::User(id) => Ok(id == user.as_i64()),
        Audience::Server(id) => Ok(member_repo::get_member(pool, user, Snowflake::new(id))
            .await?
            .is_some()),
        Audience::Dm(id) => Ok(dm_repo::is_dm_member(pool, Snowflake::new(id), user).await?),
        Audience::Channel(id) | Audience::MlsChannel(id) => {
            let server: Option<i64> =
                sqlx::query_scalar("SELECT server_id FROM channels WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await?;
            let Some(server_id) = server else {
                return Ok(false);
            };
            // Check membership explicitly even for the owner bypass in permission computation.
            if member_repo::get_member(pool, user, Snowflake::new(server_id))
                .await?
                .is_none()
            {
                return Ok(false);
            }
            Ok(effective_channel_perms(pool, user, Snowflake::new(id))
                .await?
                .contains(Permissions::VIEW_CHANNEL))
        }
    }
}

#[cfg(test)]
mod tests;
