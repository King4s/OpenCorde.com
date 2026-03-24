//! Stage session management functions.

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Stage session with topic and organizer info.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StageSessionRow {
    pub id: i64,
    pub channel_id: i64,
    pub topic: Option<String>,
    pub started_by: i64,
    pub started_at: DateTime<Utc>,
}

/// Start a new stage session on a channel.
///
/// Only one active session per channel allowed.
/// Creates the session and makes the starter a speaker.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `session_id` - Snowflake session ID
/// * `channel_id` - Snowflake channel ID
/// * `topic` - Optional session topic
/// * `started_by` - Snowflake ID of session creator
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn start_session(
    pool: &PgPool,
    session_id: Snowflake,
    channel_id: Snowflake,
    topic: Option<&str>,
    started_by: Snowflake,
) -> Result<StageSessionRow, sqlx::Error> {
    tracing::info!(
        session_id = %session_id,
        channel_id = %channel_id,
        topic = ?topic,
        started_by = %started_by,
        "starting stage session"
    );

    let row = sqlx::query_as::<_, StageSessionRow>(
        "INSERT INTO stage_sessions (id, channel_id, topic, started_by) \
         VALUES ($1, $2, $3, $4) \
         RETURNING *",
    )
    .bind(session_id.as_i64())
    .bind(channel_id.as_i64())
    .bind(topic)
    .bind(started_by.as_i64())
    .fetch_one(pool)
    .await?;

    // Add session starter as a speaker
    let _ = sqlx::query(
        "INSERT INTO stage_participants (id, channel_id, user_id, role) \
         VALUES ($1, $2, $3, 'speaker') \
         ON CONFLICT (channel_id, user_id) DO UPDATE \
         SET role = 'speaker', hand_raised = FALSE",
    )
    .bind(session_id.as_i64()) // reuse session ID as participant ID for starter
    .bind(channel_id.as_i64())
    .bind(started_by.as_i64())
    .execute(pool)
    .await;

    tracing::info!(
        session_id = %session_id,
        channel_id = %channel_id,
        "stage session started"
    );
    Ok(row)
}

/// Get the active stage session for a channel.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `channel_id` - Snowflake channel ID
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_session(
    pool: &PgPool,
    channel_id: Snowflake,
) -> Result<Option<StageSessionRow>, sqlx::Error> {
    sqlx::query_as::<_, StageSessionRow>(
        "SELECT * FROM stage_sessions WHERE channel_id = $1",
    )
    .bind(channel_id.as_i64())
    .fetch_optional(pool)
    .await
}

/// End the stage session for a channel.
///
/// Deletes the session and all associated participants.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `channel_id` - Snowflake channel ID
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn end_session(pool: &PgPool, channel_id: Snowflake) -> Result<(), sqlx::Error> {
    tracing::info!(channel_id = %channel_id, "ending stage session");

    sqlx::query("DELETE FROM stage_sessions WHERE channel_id = $1")
        .bind(channel_id.as_i64())
        .execute(pool)
        .await?;

    tracing::info!(channel_id = %channel_id, "stage session ended");
    Ok(())
}
