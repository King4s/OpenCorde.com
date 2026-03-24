//! # Route: Admin Dashboard
//! Instance administration endpoints (admin-only).
//!
//! ## Endpoints
//! - GET /api/v1/admin/stats — Instance statistics
//! - GET /api/v1/admin/users — List all users (paginated)
//! - DELETE /api/v1/admin/users/{user_id} — Delete a user account
//! - GET /api/v1/admin/servers — List all servers
//! - DELETE /api/v1/admin/servers/{server_id} — Delete a server
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db (database layer)
//! - crate::AppState (application state)
//! - crate::error::ApiError (error handling)
//! - crate::middleware::auth::AuthUser (authentication)

pub mod handlers;
pub mod types;

use axum::{
    routing::{delete, get},
    Router,
};

use crate::AppState;

pub use types::{AdminServerRow, AdminUserRow, InstanceStats, PaginationQuery};

/// Build the admin router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/admin/stats", get(handlers::get_stats))
        .route("/api/v1/admin/users", get(handlers::list_users))
        .route("/api/v1/admin/users/{user_id}", delete(handlers::delete_user))
        .route("/api/v1/admin/servers", get(handlers::list_servers))
        .route("/api/v1/admin/servers/{server_id}", delete(handlers::delete_server))
}
