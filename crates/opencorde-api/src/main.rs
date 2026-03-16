//! # OpenMesh API Server
//! Entry point for the HTTP/WebSocket server.
//!
//! ## Startup Steps
//! 1. Initialize structured logging with tracing
//! 2. Load configuration from environment variables
//! 3. Create database connection pool
//! 4. Run database migrations
//! 5. Build router with middleware
//! 6. Bind TCP listener and serve requests
//!
//! ## Environment Variables (Required)
//! - DATABASE_URL — PostgreSQL connection string
//! - JWT_SECRET — Secret for signing JWTs
//!
//! ## Environment Variables (Optional)
//! - API_HOST — Bind address (default: 127.0.0.1)
//! - API_PORT — Bind port (default: 3000)
//! - ENVIRONMENT — "production" or "development" (default: development)
//! - RUST_LOG — Tracing filter (default: debug level for api/db/tower_http)
//!
//! ## Depends On
//! - opencorde_api — Library crate with routes, middleware, etc.
//! - opencorde_db — Database layer
//! - axum — Web framework
//! - tokio — Async runtime
//! - tracing — Structured logging
//! - tower_http — Middleware implementations

use opencorde_api::{AppState, config::Config, middleware, routes};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    initialize_tracing()?;

    tracing::info!("starting OpenMesh API server");

    // Load configuration from environment
    let config = Config::from_env()?;
    tracing::info!(
        host = %config.api_host,
        port = config.api_port,
        "API configuration loaded"
    );

    // Create database connection pool
    tracing::debug!("creating database connection pool");
    let pool = opencorde_db::create_pool(&config.database_url).await?;
    tracing::debug!("database pool created");

    // Run pending migrations
    tracing::debug!("running database migrations");
    opencorde_db::run_migrations(&pool).await?;
    tracing::info!("database migrations completed");

    // Build application state
    let state = AppState {
        db: pool,
        config: Arc::new(config.clone()),
    };

    // Build router with middleware
    let is_production = state.config.is_production();
    tracing::debug!(is_production, "building application router");

    let app = routes::api_router()
        .layer(middleware::cors::cors_layer(is_production))
        .layer(middleware::make_request_id_layer())
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state.clone());

    // Bind TCP listener and start server
    let addr = format!("{}:{}", state.config.api_host, state.config.api_port);
    tracing::info!(address = %addr, "binding TCP listener");

    let listener = TcpListener::bind(&addr).await?;
    tracing::info!(address = %addr, "server listening");

    // Serve requests
    axum::serve(listener, app).await?;

    Ok(())
}

/// Initialize structured logging with tracing and tracing-subscriber.
///
/// Configures:
/// - EnvFilter from RUST_LOG environment variable with sensible defaults
/// - Format layer (JSON in production, pretty-print in development)
/// - Target and thread ID inclusion
///
/// # Errors
/// Returns anyhow::Error if tracing setup fails.
fn initialize_tracing() -> anyhow::Result<()> {
    // Create filter from RUST_LOG env var, with fallback defaults
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(
            "opencorde_api=debug,opencorde_db=debug,opencorde_core=debug,tower_http=debug,sqlx=warn",
        )
    });

    // Determine environment for output format
    let is_production = std::env::var("ENVIRONMENT")
        .map(|v| v.to_lowercase() == "production")
        .unwrap_or(false);

    if is_production {
        // Production: JSON structured logging for log aggregation
        tracing_subscriber::registry()
            .with(filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_target(true)
                    .with_thread_ids(true),
            )
            .init();
    } else {
        // Development: Pretty-printed logs for human readability
        tracing_subscriber::registry()
            .with(filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .pretty()
                    .with_target(true)
                    .with_thread_ids(true),
            )
            .init();
    }

    tracing::debug!(
        "tracing initialized (environment = {})",
        if is_production {
            "production"
        } else {
            "development"
        }
    );
    Ok(())
}
