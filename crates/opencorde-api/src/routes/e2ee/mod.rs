//! # Routes: E2EE — End-to-End Encryption Key Exchange
//!
//! ## Endpoints
//! - POST   /api/v1/users/me/key-packages               — Upload a KeyPackage bundle
//! - GET    /api/v1/users/{user_id}/key-packages/one    — Consume one KeyPackage for a user
//! - DELETE /api/v1/users/me/key-packages               — Delete all own KeyPackages (on logout)
//! - POST   /api/v1/channels/{channel_id}/e2ee/init     — Initialize E2EE group for channel
//! - GET    /api/v1/channels/{channel_id}/e2ee/welcome  — Fetch welcome message (join group)
//! - PUT    /api/v1/channels/{channel_id}/e2ee/state    — Legacy empty state acknowledgement
//!
//! ## Design
//! The server relays public key packages and encrypted Welcome messages only.
//! Private MLS group state must remain on the client.
//! All MLS cryptography happens on the client (browser/Tauri app).
//!
//! ## Depends On
//! - axum (web framework)
//! - crate::middleware::auth::AuthUser (authentication)
//! - crate::AppState (application state)

pub mod groups;
pub mod sync;
pub mod key_packages;

use crate::AppState;
use axum::Router;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(key_packages::router())
        .merge(groups::router())
        .merge(sync::router())
}
