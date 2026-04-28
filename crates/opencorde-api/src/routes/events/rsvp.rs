//! # Event RSVP Handlers
//! RSVP and un-RSVP endpoints for server events.
//!
//! ## Endpoints
//! - POST /api/v1/events/{event_id}/rsvp — RSVP to event
//! - DELETE /api/v1/events/{event_id}/rsvp — Un-RSVP
//!
//! ## Depends On
//! - opencorde_db::repos::event_repo
//! - crate::middleware::auth::AuthUser

use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use opencorde_db::repos::event_repo;
use tracing::instrument;

use super::handlers::{parse_snowflake_id, require_event_visible};
use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

/// POST /api/v1/events/{event_id}/rsvp — RSVP to an event.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub(super) async fn rsvp(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(event_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("adding rsvp");

    let event_id_sf = parse_snowflake_id(&event_id)?;
    let event = event_repo::get_by_id(&state.db, event_id_sf)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("Event not found".into()))?;
    require_event_visible(&state, &auth, &event).await?;

    event_repo::rsvp(&state.db, event_id_sf, auth.user_id)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(event_id = event_id_sf.as_i64(), "rsvp added");
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/events/{event_id}/rsvp — Un-RSVP from an event.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub(super) async fn un_rsvp(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(event_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("removing rsvp");

    let event_id_sf = parse_snowflake_id(&event_id)?;
    let event = event_repo::get_by_id(&state.db, event_id_sf)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("Event not found".into()))?;
    require_event_visible(&state, &auth, &event).await?;

    event_repo::un_rsvp(&state.db, event_id_sf, auth.user_id)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(event_id = event_id_sf.as_i64(), "rsvp removed");
    Ok(StatusCode::NO_CONTENT)
}
