//! Stage participant management functions.

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Stage participant with username joined from users table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StageParticipantRow {
    pub id: i64,
    pub channel_id: i64,
    pub user_id: i64,
    pub username: String,
    pub role: String,
    pub hand_raised: bool,
    pub joined_at: DateTime<Utc>,
}

/// Join a stage as an audience member.
///
/// Creates a participant record with audience role.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `participant_id` - Snowflake participant ID
/// * `channel_id` - Snowflake channel ID
/// * `user_id` - Snowflake user ID
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn join_stage(
    pool: &PgPool,
    participant_id: Snowflake,
    channel_id: Snowflake,
    user_id: Snowflake,
) -> Result<StageParticipantRow, sqlx::Error> {
    tracing::info!(
        channel_id = %channel_id,
        user_id = %user_id,
        "user joining stage"
    );

    let row = sqlx::query_as::<_, StageParticipantRow>(
        "INSERT INTO stage_participants (id, channel_id, user_id, role) \
         VALUES ($1, $2, $3, 'audience') \
         ON CONFLICT (channel_id, user_id) DO UPDATE \
         SET role = 'audience', hand_raised = FALSE \
         RETURNING sp.id, sp.channel_id, sp.user_id, u.username, sp.role, sp.hand_raised, sp.joined_at \
         FROM users u WHERE sp.user_id = u.id",
    )
    .bind(participant_id.as_i64())
    .bind(channel_id.as_i64())
    .bind(user_id.as_i64())
    .fetch_one(pool)
    .await?;

    tracing::info!(channel_id = %channel_id, user_id = %user_id, "user joined stage");
    Ok(row)
}

/// Leave the stage as a participant.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `channel_id` - Snowflake channel ID
/// * `user_id` - Snowflake user ID
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn leave_stage(
    pool: &PgPool,
    channel_id: Snowflake,
    user_id: Snowflake,
) -> Result<(), sqlx::Error> {
    tracing::info!(
        channel_id = %channel_id,
        user_id = %user_id,
        "user leaving stage"
    );

    sqlx::query("DELETE FROM stage_participants WHERE channel_id = $1 AND user_id = $2")
        .bind(channel_id.as_i64())
        .bind(user_id.as_i64())
        .execute(pool)
        .await?;

    tracing::info!(
        channel_id = %channel_id,
        user_id = %user_id,
        "user left stage"
    );
    Ok(())
}

/// List all participants in a stage session.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `channel_id` - Snowflake channel ID
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_participants(
    pool: &PgPool,
    channel_id: Snowflake,
) -> Result<Vec<StageParticipantRow>, sqlx::Error> {
    tracing::debug!(channel_id = %channel_id, "fetching stage participants");

    sqlx::query_as::<_, StageParticipantRow>(
        "SELECT sp.id, sp.channel_id, sp.user_id, u.username, sp.role, sp.hand_raised, sp.joined_at \
         FROM stage_participants sp \
         JOIN users u ON sp.user_id = u.id \
         WHERE sp.channel_id = $1 \
         ORDER BY sp.role DESC, sp.joined_at ASC",
    )
    .bind(channel_id.as_i64())
    .fetch_all(pool)
    .await
}

/// Raise hand to request speaking privileges.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `channel_id` - Snowflake channel ID
/// * `user_id` - Snowflake user ID
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn raise_hand(
    pool: &PgPool,
    channel_id: Snowflake,
    user_id: Snowflake,
) -> Result<(), sqlx::Error> {
    tracing::info!(channel_id = %channel_id, user_id = %user_id, "user raising hand");

    sqlx::query(
        "UPDATE stage_participants SET hand_raised = TRUE \
         WHERE channel_id = $1 AND user_id = $2",
    )
    .bind(channel_id.as_i64())
    .bind(user_id.as_i64())
    .execute(pool)
    .await?;

    tracing::info!(channel_id = %channel_id, user_id = %user_id, "hand raised");
    Ok(())
}

/// Lower hand (cancel request to speak).
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `channel_id` - Snowflake channel ID
/// * `user_id` - Snowflake user ID
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn lower_hand(
    pool: &PgPool,
    channel_id: Snowflake,
    user_id: Snowflake,
) -> Result<(), sqlx::Error> {
    tracing::info!(channel_id = %channel_id, user_id = %user_id, "user lowering hand");

    sqlx::query(
        "UPDATE stage_participants SET hand_raised = FALSE \
         WHERE channel_id = $1 AND user_id = $2",
    )
    .bind(channel_id.as_i64())
    .bind(user_id.as_i64())
    .execute(pool)
    .await?;

    tracing::info!(channel_id = %channel_id, user_id = %user_id, "hand lowered");
    Ok(())
}

/// Promote participant to speaker.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `channel_id` - Snowflake channel ID
/// * `user_id` - Snowflake user ID
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn promote_to_speaker(
    pool: &PgPool,
    channel_id: Snowflake,
    user_id: Snowflake,
) -> Result<(), sqlx::Error> {
    tracing::info!(
        channel_id = %channel_id,
        user_id = %user_id,
        "promoting to speaker"
    );

    sqlx::query(
        "UPDATE stage_participants SET role = 'speaker', hand_raised = FALSE \
         WHERE channel_id = $1 AND user_id = $2",
    )
    .bind(channel_id.as_i64())
    .bind(user_id.as_i64())
    .execute(pool)
    .await?;

    tracing::info!(
        channel_id = %channel_id,
        user_id = %user_id,
        "promoted to speaker"
    );
    Ok(())
}

/// Demote participant to audience.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `channel_id` - Snowflake channel ID
/// * `user_id` - Snowflake user ID
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn demote_to_audience(
    pool: &PgPool,
    channel_id: Snowflake,
    user_id: Snowflake,
) -> Result<(), sqlx::Error> {
    tracing::info!(
        channel_id = %channel_id,
        user_id = %user_id,
        "demoting to audience"
    );

    sqlx::query(
        "UPDATE stage_participants SET role = 'audience' \
         WHERE channel_id = $1 AND user_id = $2",
    )
    .bind(channel_id.as_i64())
    .bind(user_id.as_i64())
    .execute(pool)
    .await?;

    tracing::info!(
        channel_id = %channel_id,
        user_id = %user_id,
        "demoted to audience"
    );
    Ok(())
}
