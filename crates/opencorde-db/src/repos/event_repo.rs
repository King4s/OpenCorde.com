//! # Repository: Server Events
//! CRUD for scheduled server events and RSVP operations.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Row type for reading events from the database with aggregated data.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EventRow {
    pub id: i64,
    pub server_id: i64,
    pub channel_id: Option<i64>,
    pub creator_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub location_type: String,
    pub location_name: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub status: String,
    pub cover_image_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub rsvp_count: i64,
    pub creator_username: String,
    /// JSONB recurrence rule (e.g. {"freq":"weekly","interval":1,"by_day":["mon"]})
    pub recurrence_rule: Option<serde_json::Value>,
    /// When the recurrence ends (inclusive)
    pub recurrence_end_date: Option<DateTime<Utc>>,
    /// Parent event ID for generated instances
    pub parent_event_id: Option<i64>,
    /// Whether this event is a recurring template
    pub is_recurring: bool,
}

/// Create a new event in a server.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Snowflake ID for the event
/// * `server_id` - Snowflake ID of the parent server
/// * `channel_id` - Optional Snowflake ID of associated channel
/// * `creator_id` - Snowflake ID of the event creator
/// * `title` - Event title (max 100 chars)
/// * `description` - Optional event description
/// * `location_type` - 'voice', 'external', or 'stage'
/// * `location_name` - Optional specific location name
/// * `starts_at` - Event start timestamp
/// * `ends_at` - Optional event end timestamp
/// * `recurrence_rule` - Optional JSONB recurrence rule
/// * `recurrence_end_date` - Optional recurrence end date
///
/// # Errors
/// Returns sqlx::Error if the insert fails.
#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip(pool))]
pub async fn create_event(
    pool: &PgPool,
    id: Snowflake,
    server_id: Snowflake,
    channel_id: Option<Snowflake>,
    creator_id: Snowflake,
    title: &str,
    description: Option<&str>,
    location_type: &str,
    location_name: Option<&str>,
    starts_at: DateTime<Utc>,
    ends_at: Option<DateTime<Utc>>,
    recurrence_rule: Option<&serde_json::Value>,
    recurrence_end_date: Option<DateTime<Utc>>,
) -> Result<EventRow, sqlx::Error> {
    tracing::info!(
        title = %title,
        server_id = server_id.as_i64(),
        creator_id = creator_id.as_i64(),
        "creating event"
    );

    let is_recurring = recurrence_rule.is_some();

    sqlx::query(
        "INSERT INTO server_events \
         (id, server_id, channel_id, creator_id, title, description, location_type, location_name, \
          starts_at, ends_at, recurrence_rule, recurrence_end_date, is_recurring) \
         VALUES ($1, $2, $3, $4, $5, $6, $7::event_location_type, $8, $9, $10, $11, $12, $13)"
    )
    .bind(id.as_i64())
    .bind(server_id.as_i64())
    .bind(channel_id.map(|sf| sf.as_i64()))
    .bind(creator_id.as_i64())
    .bind(title)
    .bind(description)
    .bind(location_type)
    .bind(location_name)
    .bind(starts_at)
    .bind(ends_at)
    .bind(recurrence_rule)
    .bind(recurrence_end_date)
    .bind(is_recurring)
    .execute(pool)
    .await?;

    let event = get_by_id(pool, id).await?.ok_or(sqlx::Error::RowNotFound)?;

    tracing::info!(event_id = event.id, "event created successfully");
    Ok(event)
}

/// Insert a generated recurring event instance into `event_instances`.
///
/// # Errors
/// Returns sqlx::Error if the insert fails.
#[tracing::instrument(skip(pool))]
pub async fn create_event_instance(
    pool: &PgPool,
    id: Snowflake,
    parent_event_id: Snowflake,
    server_id: Snowflake,
    starts_at: DateTime<Utc>,
    ends_at: Option<DateTime<Utc>>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO event_instances (id, parent_event_id, server_id, starts_at, ends_at) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT DO NOTHING"
    )
    .bind(id.as_i64())
    .bind(parent_event_id.as_i64())
    .bind(server_id.as_i64())
    .bind(starts_at)
    .bind(ends_at)
    .execute(pool)
    .await?;
    Ok(())
}

/// List all instances of a recurring event.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_instances(
    pool: &PgPool,
    parent_event_id: Snowflake,
) -> Result<Vec<EventInstanceRow>, sqlx::Error> {
    sqlx::query_as::<_, EventInstanceRow>(
        "SELECT id, parent_event_id, server_id, starts_at, ends_at, status::text, created_at \
         FROM event_instances \
         WHERE parent_event_id = $1 \
         ORDER BY starts_at ASC",
    )
    .bind(parent_event_id.as_i64())
    .fetch_all(pool)
    .await
}

/// Row type for event_instances table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EventInstanceRow {
    pub id: i64,
    pub parent_event_id: i64,
    pub server_id: i64,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// Get an event by its Snowflake ID.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_by_id(pool: &PgPool, id: Snowflake) -> Result<Option<EventRow>, sqlx::Error> {
    let row = sqlx::query_as::<_, EventRow>(
        "SELECT e.id, e.server_id, e.channel_id, e.creator_id, e.title, e.description, \
                e.location_type::text, e.location_name, e.starts_at, e.ends_at, \
                e.status::text, e.cover_image_url, e.created_at, e.updated_at, \
                COUNT(r.user_id)::bigint as rsvp_count, \
                u.username as creator_username \
         FROM server_events e \
         JOIN users u ON e.creator_id = u.id \
         LEFT JOIN event_rsvps r ON e.id = r.event_id \
         WHERE e.id = $1 \
         GROUP BY e.id, u.username",
    )
    .bind(id.as_i64())
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// List all events in a server, optionally filtering out completed and cancelled events.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `server_id` - Snowflake ID of the server
/// * `include_past` - If false, excludes completed and cancelled events
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_by_server(
    pool: &PgPool,
    server_id: Snowflake,
    include_past: bool,
) -> Result<Vec<EventRow>, sqlx::Error> {
    tracing::info!(
        server_id = server_id.as_i64(),
        include_past = include_past,
        "listing server events"
    );

    let rows = if include_past {
        sqlx::query_as::<_, EventRow>(
            "SELECT e.id, e.server_id, e.channel_id, e.creator_id, e.title, e.description, \
                    e.location_type::text, e.location_name, e.starts_at, e.ends_at, \
                    e.status::text, e.cover_image_url, e.created_at, e.updated_at, \
                    COUNT(r.user_id)::bigint as rsvp_count, \
                    u.username as creator_username \
             FROM server_events e \
             JOIN users u ON e.creator_id = u.id \
             LEFT JOIN event_rsvps r ON e.id = r.event_id \
             WHERE e.server_id = $1 \
             GROUP BY e.id, u.username \
             ORDER BY e.starts_at ASC",
        )
        .bind(server_id.as_i64())
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, EventRow>(
            "SELECT e.id, e.server_id, e.channel_id, e.creator_id, e.title, e.description, \
                    e.location_type::text, e.location_name, e.starts_at, e.ends_at, \
                    e.status::text, e.cover_image_url, e.created_at, e.updated_at, \
                    COUNT(r.user_id)::bigint as rsvp_count, \
                    u.username as creator_username \
             FROM server_events e \
             JOIN users u ON e.creator_id = u.id \
             LEFT JOIN event_rsvps r ON e.id = r.event_id \
             WHERE e.server_id = $1 AND e.status != 'completed' AND e.status != 'cancelled' \
             GROUP BY e.id, u.username \
             ORDER BY e.starts_at ASC",
        )
        .bind(server_id.as_i64())
        .fetch_all(pool)
        .await?
    };

    tracing::info!(count = rows.len(), "events fetched");
    Ok(rows)
}

/// Update an event's status.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Snowflake ID of the event
/// * `status` - New status: 'active', 'completed', or 'cancelled'
///
/// # Errors
/// Returns sqlx::Error if the update fails.
#[tracing::instrument(skip(pool))]
pub async fn update_status(pool: &PgPool, id: Snowflake, status: &str) -> Result<(), sqlx::Error> {
    tracing::info!(event_id = id.as_i64(), status = %status, "updating event status");

    sqlx::query(
        "UPDATE server_events SET status = $1::event_status, updated_at = NOW() WHERE id = $2",
    )
    .bind(status)
    .bind(id.as_i64())
    .execute(pool)
    .await?;

    Ok(())
}

/// Delete an event by ID.
///
/// # Errors
/// Returns sqlx::Error if the delete fails.
#[tracing::instrument(skip(pool))]
pub async fn delete_event(pool: &PgPool, id: Snowflake) -> Result<(), sqlx::Error> {
    tracing::info!(event_id = id.as_i64(), "deleting event");

    sqlx::query("DELETE FROM server_events WHERE id = $1")
        .bind(id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

/// Add a user's RSVP to an event.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `event_id` - Snowflake ID of the event
/// * `user_id` - Snowflake ID of the user
///
/// # Errors
/// Returns sqlx::Error if the insert fails (handles duplicates with ON CONFLICT).
#[tracing::instrument(skip(pool))]
pub async fn rsvp(
    pool: &PgPool,
    event_id: Snowflake,
    user_id: Snowflake,
) -> Result<(), sqlx::Error> {
    tracing::info!(
        event_id = event_id.as_i64(),
        user_id = user_id.as_i64(),
        "adding rsvp"
    );

    sqlx::query(
        "INSERT INTO event_rsvps (event_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(event_id.as_i64())
    .bind(user_id.as_i64())
    .execute(pool)
    .await?;

    Ok(())
}

/// Remove a user's RSVP from an event.
///
/// # Errors
/// Returns sqlx::Error if the delete fails.
#[tracing::instrument(skip(pool))]
pub async fn un_rsvp(
    pool: &PgPool,
    event_id: Snowflake,
    user_id: Snowflake,
) -> Result<(), sqlx::Error> {
    tracing::info!(
        event_id = event_id.as_i64(),
        user_id = user_id.as_i64(),
        "removing rsvp"
    );

    sqlx::query("DELETE FROM event_rsvps WHERE event_id = $1 AND user_id = $2")
        .bind(event_id.as_i64())
        .bind(user_id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

/// Check if a user has RSVP'd to an event.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_rsvp_status(
    pool: &PgPool,
    event_id: Snowflake,
    user_id: Snowflake,
) -> Result<bool, sqlx::Error> {
    let result: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM event_rsvps WHERE event_id = $1 AND user_id = $2)",
    )
    .bind(event_id.as_i64())
    .bind(user_id.as_i64())
    .fetch_one(pool)
    .await?;

    Ok(result.0)
}

// ---------------------------------------------------------------------------
// Lifecycle queries — used by the event_lifecycle background worker.
// ---------------------------------------------------------------------------

/// Lightweight row for the lifecycle worker — only fields needed for
/// transitions and reminder dispatch.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LifecycleEventRow {
    pub id: i64,
    pub server_id: i64,
    pub channel_id: Option<i64>,
    pub creator_id: i64,
    pub title: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub status: String,
}

/// Fetch scheduled events that start within the next `window_minutes` minutes
/// and have not yet been reminded.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn fetch_events_to_remind(
    pool: &PgPool,
    window_minutes: i64,
) -> Result<Vec<LifecycleEventRow>, sqlx::Error> {
    sqlx::query_as::<_, LifecycleEventRow>(
        "SELECT id, server_id, channel_id, creator_id, title, starts_at, ends_at, status::text \
         FROM server_events \
         WHERE status = 'scheduled' \
           AND reminded_at IS NULL \
           AND starts_at <= NOW() + ($1 || ' minutes')::INTERVAL \
           AND starts_at > NOW() \
         ORDER BY starts_at ASC",
    )
    .bind(window_minutes)
    .fetch_all(pool)
    .await
}

/// Fetch scheduled events whose `starts_at` has passed and should be
/// transitioned to `active`.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn fetch_events_to_activate(
    pool: &PgPool,
) -> Result<Vec<LifecycleEventRow>, sqlx::Error> {
    sqlx::query_as::<_, LifecycleEventRow>(
        "SELECT id, server_id, channel_id, creator_id, title, starts_at, ends_at, status::text \
         FROM server_events \
         WHERE status = 'scheduled' AND starts_at <= NOW() \
         ORDER BY starts_at ASC",
    )
    .fetch_all(pool)
    .await
}

/// Fetch active events whose `ends_at` has passed and should be transitioned
/// to `completed`. Events without an `ends_at` are skipped (they must be
/// manually completed or cancelled).
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn fetch_events_to_complete(
    pool: &PgPool,
) -> Result<Vec<LifecycleEventRow>, sqlx::Error> {
    sqlx::query_as::<_, LifecycleEventRow>(
        "SELECT id, server_id, channel_id, creator_id, title, starts_at, ends_at, status::text \
         FROM server_events \
         WHERE status = 'active' AND ends_at IS NOT NULL AND ends_at <= NOW() \
         ORDER BY ends_at ASC",
    )
    .fetch_all(pool)
    .await
}

/// Mark an event as reminded (sets `reminded_at = NOW()`).
///
/// # Errors
/// Returns sqlx::Error if the update fails.
#[tracing::instrument(skip(pool))]
pub async fn mark_reminded(pool: &PgPool, id: Snowflake) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE server_events SET reminded_at = NOW() WHERE id = $1")
        .bind(id.as_i64())
        .execute(pool)
        .await?;
    Ok(())
}

/// Fetch the user IDs of all server members who should receive an event
/// reminder — i.e. every member with `event_reminders_enabled = true` (or no
/// row yet, which defaults to true).
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn fetch_reminder_recipients(
    pool: &PgPool,
    server_id: Snowflake,
) -> Result<Vec<i64>, sqlx::Error> {
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT sm.user_id FROM server_members sm \
         LEFT JOIN user_notification_settings uns ON sm.user_id = uns.user_id \
         WHERE sm.server_id = $1 \
           AND (uns.event_reminders_enabled IS NULL OR uns.event_reminders_enabled = true)",
    )
    .bind(server_id.as_i64())
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}
