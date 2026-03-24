//! # Server Route Handlers
//! HTTP request handlers for server endpoints.
//!
//! ## Endpoints
//! - POST /api/v1/servers — Create server
//! - GET /api/v1/servers — List user's servers
//! - GET /api/v1/servers/{id} — Get server details
//! - PATCH /api/v1/servers/{id} — Update server
//! - DELETE /api/v1/servers/{id} — Delete server

mod crud;

pub use crud::router;
