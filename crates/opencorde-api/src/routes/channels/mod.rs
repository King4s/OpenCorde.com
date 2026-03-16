//! # Route: Channels
//! Channel CRUD operations within servers.
//!
//! ## Endpoints
//! - POST /api/v1/servers/{id}/channels — Create channel
//! - GET /api/v1/servers/{id}/channels — List channels in server
//! - PATCH /api/v1/channels/{id} — Update channel
//! - DELETE /api/v1/channels/{id} — Delete channel
//!
//! ## Depends On
//! - opencorde_db::repos::channel_repo
//! - opencorde_core::Snowflake

mod handlers;
mod types;
mod validation;

pub use handlers::router;
