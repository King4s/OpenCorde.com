//! # Middleware Module
//! Axum middleware layers and extractors for request processing.
//!
//! ## Modules
//! - `auth` — JWT authentication extractor (AuthUser)
//! - `request_id` — UUID generation and propagation per request
//! - `cors` — Cross-Origin Resource Sharing configuration
//!
//! ## Depends On
//! - axum (web framework and extractor traits)
//! - tower_http (middleware implementations)

pub mod auth;
pub mod cors;
pub mod request_id;

pub use auth::AuthUser;
pub use cors::cors_layer;
pub use request_id::make_request_id_layer;
