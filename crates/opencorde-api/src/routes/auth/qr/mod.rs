//! # Route: QR Code Authentication
//! Device handoff via QR code scanning.
//!
//! ## Endpoints
//! - POST /api/v1/auth/qr/request — Generate a short-lived QR login token
//! - POST /api/v1/auth/qr/confirm — Mobile device confirms QR login
//! - GET  /api/v1/auth/qr/{token} — Poll for approval (returns JWT on success)
//!
//! ## Flow
//! 1. Desktop (unauthenticated) calls POST /qr/request → gets token
//! 2. Desktop shows QR code, polls GET /qr/{token} every 2s
//! 3. Mobile (authenticated) scans QR, calls POST /qr/confirm with token
//! 4. Server stores JWT under token key in Redis
//! 5. Desktop poll returns JWT → logs in
//!
//! ## Depends On
//! - axum (web framework)
//! - redis (ephemeral token storage, 5-min TTL)
//! - crate::jwt (token creation)
//! - crate::AppState (database + config + redis)
//! - crate::error::ApiError (unified error handling)

mod confirm;
mod poll;
mod request;

use crate::AppState;
use axum::{
    Router,
    routing::{get, post},
};

/// Build the QR auth router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/qr/request", post(request::request_qr))
        .route("/api/v1/auth/qr/confirm", post(confirm::confirm_qr))
        .route("/api/v1/auth/qr/{token}", get(poll::poll_qr))
}
