//! # Repository: Voice States
//! Manages active voice channel connections and state.
//!
//! Provides functions to join/leave voice channels, update mute/deafen state,
//! and query participants.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake
//! - sqlx (database driver)
//! - chrono (DateTime handling)

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Row type for reading voice states from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct VoiceStateRow {
    pub user_id: i64,
    pub channel_id: i64,
    pub session_id: String,
    pub self_mute: bool,
    pub self_deaf: bool,
    pub joined_at: DateTime<Utc>,
}

/// Join or update a user's voice channel connection.
///
/// If the user is already in a voice channel, their channel is updated.
/// Otherwise, a new voice state is inserted.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `user_id` - Snowflake user ID
/// * `channel_id` - Snowflake channel ID (must be voice type)
/// * `session_id` - Unique session identifier (e.g., UUID)
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn join_voice(
    pool: &PgPool,
    user_id: Snowflake,
    channel_id: Snowflake,
    session_id: &str,
) -> Result<VoiceStateRow, sqlx::Error> {
    tracing::info!(
        user_id = %user_id,
        channel_id = %channel_id,
        session_id = %session_id,
        "user joining voice channel"
    );

    let row = sqlx::query_as::<_, VoiceStateRow>(
        "INSERT INTO voice_states (user_id, channel_id, session_id, self_mute, self_deaf) \
         VALUES ($1, $2, $3, false, false) \
         ON CONFLICT (user_id) DO UPDATE \
         SET channel_id = EXCLUDED.channel_id, session_id = EXCLUDED.session_id, joined_at = NOW() \
         RETURNING *",
    )
    .bind(user_id.as_i64())
    .bind(channel_id.as_i64())
    .bind(session_id)
    .fetch_one(pool)
    .await?;

    tracing::info!(user_id = %user_id, channel_id = %channel_id, "voice state created");
    Ok(row)
}

/// Leave the voice channel for a user.
///
/// Removes the user's voice state from the database.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `user_id` - Snowflake user ID
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn leave_voice(pool: &PgPool, user_id: Snowflake) -> Result<(), sqlx::Error> {
    tracing::info!(user_id = %user_id, "user leaving voice channel");

    sqlx::query("DELETE FROM voice_states WHERE user_id = $1")
        .bind(user_id.as_i64())
        .execute(pool)
        .await?;

    tracing::info!(user_id = %user_id, "voice state deleted");
    Ok(())
}

/// Update a user's voice state (mute/deafen status).
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `user_id` - Snowflake user ID
/// * `self_mute` - Whether user has muted their microphone
/// * `self_deaf` - Whether user has deafened themselves
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn update_state(
    pool: &PgPool,
    user_id: Snowflake,
    self_mute: bool,
    self_deaf: bool,
) -> Result<VoiceStateRow, sqlx::Error> {
    tracing::info!(
        user_id = %user_id,
        self_mute = self_mute,
        self_deaf = self_deaf,
        "updating voice state"
    );

    let row = sqlx::query_as::<_, VoiceStateRow>(
        "UPDATE voice_states SET self_mute = $2, self_deaf = $3 WHERE user_id = $1 RETURNING *",
    )
    .bind(user_id.as_i64())
    .bind(self_mute)
    .bind(self_deaf)
    .fetch_one(pool)
    .await?;

    tracing::info!(user_id = %user_id, "voice state updated");
    Ok(row)
}

/// Get the current voice state for a user.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `user_id` - Snowflake user ID
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_by_user(
    pool: &PgPool,
    user_id: Snowflake,
) -> Result<Option<VoiceStateRow>, sqlx::Error> {
    sqlx::query_as::<_, VoiceStateRow>("SELECT * FROM voice_states WHERE user_id = $1")
        .bind(user_id.as_i64())
        .fetch_optional(pool)
        .await
}

/// List all participants in a voice channel.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `channel_id` - Snowflake channel ID
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_channel_participants(
    pool: &PgPool,
    channel_id: Snowflake,
) -> Result<Vec<VoiceStateRow>, sqlx::Error> {
    tracing::debug!(channel_id = %channel_id, "fetching voice channel participants");

    sqlx::query_as::<_, VoiceStateRow>(
        "SELECT * FROM voice_states WHERE channel_id = $1 ORDER BY joined_at ASC",
    )
    .bind(channel_id.as_i64())
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_state_row_creation() {
        let row = VoiceStateRow {
            user_id: 123456789,
            channel_id: 987654321,
            session_id: "test-session".to_string(),
            self_mute: false,
            self_deaf: false,
            joined_at: Utc::now(),
        };

        assert_eq!(row.user_id, 123456789);
        assert_eq!(row.channel_id, 987654321);
        assert_eq!(row.session_id, "test-session");
        assert!(!row.self_mute);
        assert!(!row.self_deaf);
    }

    #[test]
    fn test_voice_state_row_muted() {
        let row = VoiceStateRow {
            user_id: 111,
            channel_id: 222,
            session_id: "session".to_string(),
            self_mute: true,
            self_deaf: true,
            joined_at: Utc::now(),
        };

        assert!(row.self_mute);
        assert!(row.self_deaf);
    }

    #[test]
    fn test_voice_state_row_clone() {
        let row1 = VoiceStateRow {
            user_id: 999,
            channel_id: 888,
            session_id: "test".to_string(),
            self_mute: false,
            self_deaf: false,
            joined_at: Utc::now(),
        };

        let row2 = row1.clone();
        assert_eq!(row1.user_id, row2.user_id);
        assert_eq!(row1.session_id, row2.session_id);
    }
}
