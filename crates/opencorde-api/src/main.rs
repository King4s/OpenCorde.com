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
use tokio::sync::broadcast;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use aws_sdk_s3::config::{Credentials as S3Credentials, Region as S3Region};

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

    // Initialize search engine (optional)
    let search = None; // TODO: Initialize search engine from config
    tracing::debug!("search engine initialized: {:?}", search.is_some());

    // Initialize S3 client for MinIO/AWS S3 object storage
    tracing::debug!("initializing S3 client");
    let s3_creds = S3Credentials::new(
        &config.minio_access_key,
        &config.minio_secret_key,
        None, None, "opencorde",
    );
    let s3_config = aws_sdk_s3::Config::builder()
        .credentials_provider(s3_creds)
        .region(S3Region::new("us-east-1"))
        .endpoint_url(&config.minio_endpoint)
        .force_path_style(true)
        .behavior_version_latest()
        .build();
    let s3_client = aws_sdk_s3::Client::from_conf(s3_config);
    tracing::debug!("S3 client initialized");

    // Create event broadcast channel (capacity 1024 events)
    // REST handlers publish events here; WebSocket connections subscribe.
    let (event_tx, _) = broadcast::channel::<serde_json::Value>(1024);
    let event_tx = Arc::new(event_tx);
    tracing::debug!("event broadcast channel created");

    // Initialize email service
    tracing::debug!("initializing email service");
    let email_service = opencorde_api::email::EmailService::new(
        config.smtp_host.clone(),
        config.smtp_port,
        config.smtp_username.clone(),
        config.smtp_password.clone(),
        config.smtp_from.clone(),
        config.base_url.clone(),
    );
    tracing::debug!(
        smtp_configured = %email_service.is_configured(),
        "email service initialized"
    );

    // Build application state
    let state = AppState {
        db: pool,
        config: Arc::new(config.clone()),
        search,
        s3: Arc::new(s3_client),
        email_service,
        event_tx,
    };

    // Build router with middleware
    let is_production = state.config.is_production();
    tracing::debug!(is_production, "building application router");

    // Request timeout (30s) and body size limit (16 MB) from config
    let request_timeout_secs = std::env::var("REQUEST_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(30);
    let body_limit_bytes = std::env::var("BODY_LIMIT_BYTES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(16 * 1024 * 1024); // 16 MB

    tracing::info!(
        request_timeout_secs,
        body_limit_bytes,
        "rate limiting and request limits configured"
    );

    let app = routes::api_router()
        .layer(tower_http::limit::RequestBodyLimitLayer::new(body_limit_bytes))
        .layer(tower_http::timeout::TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            std::time::Duration::from_secs(request_timeout_secs),
        ))
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
