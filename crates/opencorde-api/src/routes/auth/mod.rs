//! # Route: Authentication
//! User registration, login, and token refresh endpoints.
//!
//! ## Endpoints
//! - POST /api/v1/auth/register — Create new account
//! - POST /api/v1/auth/login — Login with email + password
//! - POST /api/v1/auth/refresh — Refresh access token via refresh_token cookie
//!
//! ## Features
//! - Input validation (username, email, password format)
//! - Conflict detection (duplicate email/username)
//! - Argon2id password hashing
//! - JWT access + refresh token generation
//! - HttpOnly refresh token cookie
//! - Comprehensive structured logging
//!
//! ## Modules
//! - `types` — Request/response types
//! - `handlers` — HTTP request handlers
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db::repos::user_repo (CRUD operations)
//! - opencorde_core::password (password hashing)
//! - opencorde_core::Snowflake (ID generation)
//! - crate::jwt (token creation)
//! - crate::AppState (database + config)
//! - crate::error::ApiError (unified error handling)

mod handlers;
mod register;
mod types;
mod validation;

use crate::AppState;
use axum::{Router, routing::post};

pub use types::{AuthResponse, LoginRequest, RegisterRequest, UserInfo};

/// Build the authentication router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/register", post(register::register))
        .route("/api/v1/auth/login", post(handlers::login))
        .route("/api/v1/auth/refresh", post(handlers::refresh))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let _router = router();
    }
}
