//! # Route: Delete Account
//! GDPR right-to-erasure: permanently delete authenticated user's account and all personal data.
//!
//! ## Endpoint
//! - DELETE /api/v1/users/@me — Delete account (requires password confirmation)
//!
//! ## Flow
//! 1. Verify password matches stored hash
//! 2. Reject if user owns any servers (must transfer/delete servers first)
//! 3. In a transaction, remove all rows referencing the user (non-CASCADE FKs)
//! 4. Delete the user row (CASCADE handles remaining references)
//!
//! ## Depends On
//! - axum (web framework)
//! - sqlx (database + transactions)
//! - opencorde_core::password (password verification)
//! - opencorde_db::repos::user_repo (user lookup)
//! - crate::AppState, crate::middleware::auth::AuthUser

use axum::{extract::State, http::StatusCode, Json};
use opencorde_core::password;
use opencorde_db::repos::user_repo;
use serde::Deserialize;

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};

/// Request body for account deletion.
#[derive(Debug, Deserialize)]
pub struct DeleteAccountRequest {
    /// Current password — required to confirm intentional deletion.
    pub password: String,
}

/// DELETE /api/v1/users/@me — Permanently delete the authenticated user's account.
///
/// All personal data is removed: messages, files, memberships, friendships.
/// The user must not own any servers (transfer or delete them first).
#[tracing::instrument(skip(state, auth, payload))]
pub async fn delete_account(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<DeleteAccountRequest>,
) -> Result<StatusCode, ApiError> {
    let uid = auth.user_id.as_i64();
    tracing::info!(user_id = uid, "account deletion requested");

    // 1. Fetch user to verify password
    let user_row = user_repo::get_by_id(&state.db, auth.user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("user not found".into()))?;

    let hash = user_row
        .password_hash
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("no password set on account".into()))?;

    let valid = password::verify_password(&payload.password, hash)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("password check failed: {e}")))?;

    if !valid {
        tracing::warn!(user_id = uid, "account deletion rejected: wrong password");
        return Err(ApiError::Unauthorized);
    }

    // 2. Reject if user owns servers (they must transfer/delete first)
    let owned: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM servers WHERE owner_id = $1")
        .bind(uid)
        .fetch_one(&state.db)
        .await
        .map_err(ApiError::Database)?;

    if owned > 0 {
        return Err(ApiError::BadRequest(
            "transfer or delete your servers before deleting your account".into(),
        ));
    }

    // 3. Delete all user data in dependency order (handles non-CASCADE FKs)
    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;

    // Ephemeral stage sessions this user started
    sqlx::query("DELETE FROM stage_sessions WHERE started_by = $1")
        .bind(uid).execute(&mut *tx).await.map_err(ApiError::Database)?;

    // Server config items created by user (server stays, creator ref must go)
    sqlx::query("DELETE FROM automod_rules WHERE created_by = $1")
        .bind(uid).execute(&mut *tx).await.map_err(ApiError::Database)?;
    sqlx::query("DELETE FROM slash_commands WHERE created_by = $1")
        .bind(uid).execute(&mut *tx).await.map_err(ApiError::Database)?;
    sqlx::query("DELETE FROM webhooks WHERE created_by = $1")
        .bind(uid).execute(&mut *tx).await.map_err(ApiError::Database)?;
    sqlx::query("DELETE FROM server_emojis WHERE uploaded_by = $1")
        .bind(uid).execute(&mut *tx).await.map_err(ApiError::Database)?;

    // Events (cascades event_rsvps for other users)
    sqlx::query("DELETE FROM events WHERE creator_id = $1")
        .bind(uid).execute(&mut *tx).await.map_err(ApiError::Database)?;

    // Forum content (delete replies first, then posts, or cascade handles it)
    sqlx::query("DELETE FROM forum_replies WHERE author_id = $1")
        .bind(uid).execute(&mut *tx).await.map_err(ApiError::Database)?;
    sqlx::query("DELETE FROM forum_posts WHERE author_id = $1")
        .bind(uid).execute(&mut *tx).await.map_err(ApiError::Database)?;

    // Threads and pins
    sqlx::query("DELETE FROM threads WHERE created_by = $1")
        .bind(uid).execute(&mut *tx).await.map_err(ApiError::Database)?;
    sqlx::query("DELETE FROM pinned_messages WHERE pinned_by = $1")
        .bind(uid).execute(&mut *tx).await.map_err(ApiError::Database)?;

    // Invites and voice presence
    sqlx::query("DELETE FROM invites WHERE creator_id = $1")
        .bind(uid).execute(&mut *tx).await.map_err(ApiError::Database)?;
    sqlx::query("DELETE FROM voice_states WHERE user_id = $1")
        .bind(uid).execute(&mut *tx).await.map_err(ApiError::Database)?;

    // Bans this user issued (bans where this user IS banned have CASCADE)
    sqlx::query("DELETE FROM bans WHERE banned_by = $1")
        .bind(uid).execute(&mut *tx).await.map_err(ApiError::Database)?;

    // Files and messages
    sqlx::query("DELETE FROM files WHERE uploader_id = $1")
        .bind(uid).execute(&mut *tx).await.map_err(ApiError::Database)?;
    sqlx::query("DELETE FROM dm_messages WHERE author_id = $1")
        .bind(uid).execute(&mut *tx).await.map_err(ApiError::Database)?;
    sqlx::query("DELETE FROM messages WHERE author_id = $1")
        .bind(uid).execute(&mut *tx).await.map_err(ApiError::Database)?;

    // Server memberships
    sqlx::query("DELETE FROM server_members WHERE user_id = $1")
        .bind(uid).execute(&mut *tx).await.map_err(ApiError::Database)?;

    // Delete user row (CASCADE cleans up reactions, read_state, dm_channel_members,
    // relationships, stage_participants, e2ee keys, bridge ghost, password_reset_tokens;
    // audit_log.actor_id SET NULL)
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(uid).execute(&mut *tx).await.map_err(ApiError::Database)?;

    tx.commit().await.map_err(ApiError::Database)?;

    tracing::info!(user_id = uid, "account deleted successfully");
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delete_account_request_deserialization() {
        let json = r#"{"password":"hunter2"}"#;
        let req: DeleteAccountRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.password, "hunter2");
    }
}
