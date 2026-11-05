//! Transaction-local authorization: never borrow an extra pool connection.
use super::*;
use opencorde_core::permissions::{OverwriteType, PermissionOverwrite, compute_permissions};
pub(super) async fn require(
    connection: &mut sqlx::PgConnection,
    user: Snowflake,
    channel: Snowflake,
    required: Permissions,
    send: bool,
) -> Result<(), ApiError> {
    let server=sqlx::query("SELECT s.id,s.owner_id,s.verification_level FROM servers s JOIN channels c ON c.server_id=s.id WHERE c.id=$1 FOR SHARE OF s")
 .bind(channel.as_i64()).fetch_optional(&mut *connection).await?.ok_or_else(||ApiError::NotFound("channel not found".into()))?;
    let server_id: i64 = server.get("id");
    let owner: i64 = server.get("owner_id");
    let membership = sqlx::query(
        "SELECT joined_at FROM server_members WHERE server_id=$1 AND user_id=$2 FOR SHARE",
    )
    .bind(server_id)
    .bind(user.as_i64())
    .fetch_optional(&mut *connection)
    .await?;
    if owner != user.as_i64() && membership.is_none() {
        return Err(ApiError::Forbidden);
    }
    if owner != user.as_i64() {
        let roles=sqlx::query("SELECT r.id,r.permissions FROM roles r JOIN member_roles m ON m.role_id=r.id WHERE m.server_id=$1 AND m.user_id=$2 AND r.server_id=$1 FOR SHARE OF r,m").bind(server_id).bind(user.as_i64()).fetch_all(&mut *connection).await?;
        let role_ids: Vec<_> = roles.iter().map(|r| Snowflake::new(r.get("id"))).collect();
        let base = roles.iter().fold(Permissions::default_everyone(), |p, r| {
            p | Permissions::from_bits_truncate(r.get::<i64, _>("permissions") as u64)
        });
        let rows=sqlx::query("SELECT target_id,target_type,allow_bits,deny_bits FROM channel_permission_overrides WHERE channel_id=$1 FOR SHARE").bind(channel.as_i64()).fetch_all(&mut *connection).await?;
        let overwrites: Vec<_> = rows
            .iter()
            .map(|r| PermissionOverwrite {
                id: Snowflake::new(r.get("target_id")),
                target_type: if r.get::<String, _>("target_type") == "role" {
                    OverwriteType::Role
                } else {
                    OverwriteType::Member
                },
                allow: Permissions::from_bits_truncate(r.get::<i64, _>("allow_bits") as u64),
                deny: Permissions::from_bits_truncate(r.get::<i64, _>("deny_bits") as u64),
            })
            .collect();
        let effective = if base.contains(Permissions::ADMINISTRATOR) {
            Permissions::all()
        } else {
            compute_permissions(
                base,
                &overwrites,
                user,
                &role_ids,
                Some(Snowflake::new(server_id)),
            )
        };
        if !effective.contains(required) {
            return Err(ApiError::Forbidden);
        }
        if send {
            let timed_out:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM member_timeouts WHERE server_id=$1 AND user_id=$2 AND timeout_until>CURRENT_TIMESTAMP)").bind(server_id).bind(user.as_i64()).fetch_one(&mut *connection).await?;
            if timed_out {
                return Err(ApiError::Forbidden);
            }
        }
    }
    if send {
        let level: i16 = server.get("verification_level");
        if level > 0 {
            let account = sqlx::query(
                "SELECT email_verified,created_at,totp_enabled FROM users WHERE id=$1 FOR SHARE",
            )
            .bind(user.as_i64())
            .fetch_one(&mut *connection)
            .await?;
            let created: chrono::DateTime<chrono::Utc> = account.get("created_at");
            if !account.get::<bool, _>("email_verified")
                || (level >= 2
                    && chrono::Utc::now()
                        .signed_duration_since(created)
                        .num_seconds()
                        < 300)
                || (level >= 4 && !account.get::<bool, _>("totp_enabled"))
            {
                return Err(ApiError::Forbidden);
            }
            if level >= 3
                && let Some(member) = membership
            {
                let joined: chrono::DateTime<chrono::Utc> = member.get("joined_at");
                if chrono::Utc::now()
                    .signed_duration_since(joined)
                    .num_seconds()
                    < 600
                {
                    return Err(ApiError::Forbidden);
                }
            }
        }
    }
    Ok(())
}
