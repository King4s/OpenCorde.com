//! # Route: Servers
//! Server CRUD operations for server management.
//!
//! ## Endpoints
//! - POST /api/v1/servers — Create server
//! - GET /api/v1/servers — List user's servers
//! - GET /api/v1/servers/{id} — Get server details
//! - PATCH /api/v1/servers/{id} — Update server (owner only)
//! - DELETE /api/v1/servers/{id} — Delete server (owner only)
//!
//! ## Depends On
//! - opencorde_db::repos::{server_repo, member_repo}
//! - opencorde_core::Snowflake

mod handlers;
pub mod types;
mod validation;

pub use handlers::router;
