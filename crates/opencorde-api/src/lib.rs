//! # OpenMesh API
//! REST API and WebSocket gateway for OpenMesh.
//!
//! ## Features
//! - Axum-based HTTP server with middleware stack
//! - Unified error handling with structured responses
//! - Request ID propagation for distributed tracing
//! - CORS configuration (environment-aware)
//! - Health check endpoint
//! - WebSocket gateway infrastructure (Phase 1: Weeks 7-8)
//!
//! ## Modules
//! - `config` — Environment configuration loading
//! - `error` — Unified ApiError type and response formatting
//! - `routes` — HTTP route handlers (auth, servers, channels, etc.)
//! - `middleware` — Request processing layers (CORS, request ID, etc.)
//! - `ws` — WebSocket gateway (Phase 1, Weeks 7-8)
//!
//! ## Depends On
//! - opencorde_core — Model types and domain logic
//! - opencorde_db — Database layer with repository pattern
//! - axum — Web framework and routing
//! - tokio — Async runtime
//! - sqlx — Database access
//! - tower_http — Middleware implementations
//! - tracing — Structured logging
//! - uuid — Request ID generation
//!
//! ## Architecture
//! The application is organized by concern:
//! - Routes are HTTP handlers grouped by resource (servers, channels, etc.)
//! - Middleware applies cross-cutting concerns (auth, rate limiting, CORS)
//! - Config handles environment variables and secrets
//! - Error centralizes error handling and HTTP response mapping
//! - Main.rs orchestrates startup and server initialization

pub mod config;
pub mod error;
pub mod jwt;
pub mod middleware;
pub mod routes;
pub mod ws;

use axum::extract::FromRef;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Shared application state.
///
/// Contains the database pool, configuration, optional search engine,
/// and an event broadcast channel for real-time WebSocket fan-out.
/// Passed to all route handlers via Axum's State extractor.
#[derive(Clone)]
pub struct AppState {
    /// PostgreSQL connection pool
    pub db: PgPool,
    /// Application configuration (wrapped in Arc for shared ownership)
    pub config: Arc<config::Config>,
    /// Optional search engine (None if search is disabled)
    pub search: Option<Arc<opencorde_search::SearchEngine>>,
    /// Broadcast channel for pushing real-time events to WebSocket clients.
    /// REST handlers publish here; WS handler subscribes.
    pub event_tx: Arc<broadcast::Sender<serde_json::Value>>,
}

/// Allow extracting Arc<Config> from AppState.
impl FromRef<AppState> for Arc<config::Config> {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}
