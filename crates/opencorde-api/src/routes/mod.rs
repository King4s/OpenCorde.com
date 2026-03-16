//! # Routes Module
//! HTTP route handlers for the REST API.
//!
//! ## Modules
//! - `health` — Liveness/readiness probe endpoint
//! - `auth` — Authentication (register, login, refresh)
//! - `users` — User profile management
//! - `servers` — Server CRUD operations
//! - `channels` — Channel CRUD operations
//! - `messages` — Message CRUD and typing indicator
//!
//! ## Depends On
//! - axum (web framework)
//! - crate::AppState (application state)

pub mod auth;
pub mod channels;
pub mod health;
pub mod helpers;
pub mod invites;
pub mod members;
pub mod messages;
pub mod roles;
pub mod servers;
pub mod users;
pub mod voice;

use axum::Router;

use crate::AppState;

/// Build the complete API router with all routes.
pub fn api_router() -> Router<AppState> {
    Router::new()
        .merge(health::router())
        .merge(auth::router())
        .merge(users::router())
        .merge(servers::router())
        .merge(channels::router())
        .merge(invites::router())
        .merge(members::router())
        .merge(roles::router())
        .merge(messages::router())
        .merge(voice::router())
        .merge(crate::ws::handler::router())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_router_creation() {
        let _router = api_router();
        // Verify all route modules can be composed without panic
    }
}
