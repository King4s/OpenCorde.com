//! # Event Route Handlers
//! HTTP request handlers for event endpoints.
//!
//! ## Endpoints
//! - POST /api/v1/servers/{server_id}/events — Create event
//! - GET /api/v1/servers/{server_id}/events — List events
//! - GET /api/v1/events/{event_id} — Get event details
//! - PATCH /api/v1/events/{event_id} — Update event status
//! - DELETE /api/v1/events/{event_id} — Delete event
//! - POST /api/v1/events/{event_id}/rsvp — RSVP to event
//! - DELETE /api/v1/events/{event_id}/rsvp — Un-RSVP
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db::repos::event_repo
//! - opencorde_core::Snowflake
//! - crate::middleware::auth::AuthUser
//! - crate::AppState

use axum::{
    Json, Router,
    extract::{Path, State, Query},
    http::StatusCode,
    routing::{get, post},
};
use chrono::Utc;
use opencorde_db::repos::event_repo;
use std::collections::HashMap;
use tracing::instrument;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

use super::types::{EventResponse, CreateEventRequest, UpdateEventRequest};

/// Parse a Snowflake ID from a string path parameter.
pub(super) fn parse_snowflake_id(id_str: &str) -> Result<opencorde_core::Snowflake, ApiError> {
    id_str
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("invalid id format".into()))
        .and_then(|id| {
            if id > 0 {
                Ok(opencorde_core::Snowflake::new(id))
            } else {
                Err(ApiError::BadRequest("id must be positive".into()))
            }
        })
}

/// Build the events router with all CRUD endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/servers/{server_id}/events",
            post(create_event).get(list_events),
        )
        .route(
            "/api/v1/events/{event_id}",
            get(get_event).patch(update_event).delete(delete_event),
        )
        .route(
            "/api/v1/events/{event_id}/rsvp",
            post(super::rsvp::rsvp).delete(super::rsvp::un_rsvp),
        )
}

/// Convert EventRow to EventResponse.
fn event_row_to_response(row: event_repo::EventRow) -> EventResponse {
    EventResponse {
        id: row.id.to_string(),
        server_id: row.server_id.to_string(),
        channel_id: row.channel_id.map(|id| id.to_string()),
        creator_id: row.creator_id.to_string(),
        creator_username: row.creator_username,
        title: row.title,
        description: row.description,
        location_type: row.location_type,
        location_name: row.location_name,
        starts_at: row.starts_at,
        ends_at: row.ends_at,
        status: row.status,
        cover_image_url: row.cover_image_url,
        rsvp_count: row.rsvp_count,
        created_at: row.created_at,
    }
}

/// POST /api/v1/servers/{server_id}/events — Create a new event.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn create_event(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
    Json(req): Json<CreateEventRequest>,
) -> Result<(StatusCode, Json<EventResponse>), ApiError> {
    tracing::info!(title = %req.title, "creating event");

    // Parse server ID
    let server_id_sf = parse_snowflake_id(&server_id)?;

    // Validate title
    if req.title.is_empty() || req.title.len() > 100 {
        return Err(ApiError::BadRequest("Title must be 1-100 characters".to_string()));
    }

    // Validate starts_at is in the future
    if req.starts_at < Utc::now() {
        return Err(ApiError::BadRequest("Event start time must be in the future".to_string()));
    }

    // Validate ends_at is after starts_at if provided
    if let Some(ends_at) = req.ends_at {
        if ends_at < req.starts_at {
            return Err(ApiError::BadRequest("Event end time must be after start time".to_string()));
        }
    }

    // Parse channel_id if provided
    let channel_id = if let Some(ch_str) = &req.channel_id {
        Some(parse_snowflake_id(ch_str)?)
    } else {
        None
    };

    let location_type = req.location_type.unwrap_or_else(|| "external".to_string());

    // Validate location_type
    if !matches!(location_type.as_str(), "voice" | "external" | "stage") {
        return Err(ApiError::BadRequest("Invalid location_type".to_string()));
    }

    // Generate Snowflake ID for event
    let mut generator = opencorde_core::snowflake::SnowflakeGenerator::new(1, 1);
    let event_id = generator.next_id();

    tracing::debug!(event_id = event_id.as_i64(), "generated event snowflake");

    // Create event in database
    let event_row = event_repo::create_event(
        &state.db,
        event_id,
        server_id_sf,
        channel_id,
        auth.user_id,
        &req.title,
        req.description.as_deref(),
        &location_type,
        req.location_name.as_deref(),
        req.starts_at,
        req.ends_at,
    )
    .await
    .map_err(ApiError::Database)?;

    tracing::info!(event_id = event_row.id, "event created");

    let response = event_row_to_response(event_row);
    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/v1/servers/{server_id}/events — List events in a server.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn list_events(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<EventResponse>>, ApiError> {
    tracing::info!("listing server events");

    // Parse server ID
    let server_id_sf = parse_snowflake_id(&server_id)?;

    let include_past = params.get("past").map(|v| v == "true").unwrap_or(false);
    tracing::debug!(include_past = include_past, "parsed query params");

    let events = event_repo::list_by_server(&state.db, server_id_sf, include_past)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(count = events.len(), "events fetched");

    let responses: Vec<EventResponse> =
        events.into_iter().map(event_row_to_response).collect();

    Ok(Json(responses))
}

/// GET /api/v1/events/{event_id} — Get event details.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn get_event(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(event_id): Path<String>,
) -> Result<Json<EventResponse>, ApiError> {
    tracing::info!("fetching event");

    let event_id_sf = parse_snowflake_id(&event_id)?;

    let event = event_repo::get_by_id(&state.db, event_id_sf)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("Event not found".to_string()))?;

    let response = event_row_to_response(event);
    Ok(Json(response))
}

/// PATCH /api/v1/events/{event_id} — Update event status.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn update_event(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(event_id): Path<String>,
    Json(req): Json<UpdateEventRequest>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(status = %req.status, "updating event status");

    let event_id_sf = parse_snowflake_id(&event_id)?;

    // Validate status
    if !matches!(req.status.as_str(), "active" | "completed" | "cancelled") {
        return Err(ApiError::BadRequest("Invalid status".to_string()));
    }

    // Fetch event to verify creator
    let event = event_repo::get_by_id(&state.db, event_id_sf)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("Event not found".to_string()))?;

    if event.creator_id != auth.user_id.as_i64() {
        return Err(ApiError::Forbidden);
    }

    event_repo::update_status(&state.db, event_id_sf, &req.status)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(event_id = event_id_sf.as_i64(), "event status updated");
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/events/{event_id} — Delete an event.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn delete_event(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(event_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("deleting event");

    let event_id_sf = parse_snowflake_id(&event_id)?;

    // Fetch event to verify creator
    let event = event_repo::get_by_id(&state.db, event_id_sf)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("Event not found".to_string()))?;

    if event.creator_id != auth.user_id.as_i64() {
        return Err(ApiError::Forbidden);
    }

    event_repo::delete_event(&state.db, event_id_sf)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(event_id = event_id_sf.as_i64(), "event deleted");
    Ok(StatusCode::NO_CONTENT)
}

