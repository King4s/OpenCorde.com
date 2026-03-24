//! # Route: Events
//! Scheduled server events with RSVP support.
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
//! - opencorde_db::repos::event_repo
//! - opencorde_core::Snowflake

mod handlers;
mod rsvp;
mod types;

pub use handlers::router;
