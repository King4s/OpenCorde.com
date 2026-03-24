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
mod password_reset;
mod register;
mod steam;
mod steam_verify;
mod types;
mod validation;
mod verification;

use crate::AppState;
use axum::{Router, routing::{get, post}};

pub use types::{AuthResponse, LoginRequest, RegisterRequest, UserInfo};

/// Build the authentication router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/register", post(register::register))
        .route("/api/v1/auth/login", post(handlers::login))
        .route("/api/v1/auth/refresh", post(handlers::refresh))
        .route("/api/v1/auth/forgot-password", post(password_reset::forgot_password))
        .route("/api/v1/auth/reset-password", post(password_reset::reset_password))
        .route("/api/v1/auth/verify-email", get(verification::verify_email))
        .route("/api/v1/auth/resend-verification", post(verification::resend_verification))
        .route("/api/v1/auth/steam", get(steam::steam_login))
        .route("/api/v1/auth/steam/callback", get(steam::steam_callback))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let _router = router();
    }
}
