//! # OpenMesh DB
//! PostgreSQL database layer via sqlx.
//!
//! ## Features
//! - Connection pooling via sqlx PgPool
//! - Database migrations with sqlx::migrate
//! - Repository pattern for CRUD operations
//! - Structured logging via tracing
//!
//! ## Modules
//! - `repos` — Repository pattern, one repo per entity
//!
//! ## Depends On
//! - opencorde_core (model types)

pub mod repos;

use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

/// Create a PostgreSQL connection pool.
///
/// # Arguments
/// * `database_url` - PostgreSQL connection string (e.g., from DATABASE_URL env var)
///
/// # Returns
/// A configured PgPool for database operations.
///
/// # Errors
/// Returns sqlx::Error if connection fails.
#[tracing::instrument]
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    tracing::info!("creating PostgreSQL connection pool");

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await?;

    tracing::info!("PostgreSQL connection pool created");
    Ok(pool)
}

/// Run pending database migrations.
///
/// Applies all SQL migration files from the migrations/ directory.
/// Safe to call multiple times — already-applied migrations are skipped.
///
/// # Errors
/// Returns sqlx::migrate::MigrateError if migrations fail.
#[tracing::instrument(skip(pool))]
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    tracing::info!("running database migrations");
    sqlx::migrate!("./migrations").run(pool).await?;
    tracing::info!("database migrations complete");
    Ok(())
}

/// Check database health via a simple query.
///
/// Useful for startup checks and liveness probes.
///
/// # Errors
/// Returns sqlx::Error if the health check query fails.
#[tracing::instrument(skip(pool))]
pub async fn health_check(pool: &PgPool) -> Result<(), sqlx::Error> {
    tracing::debug!("checking database health");
    sqlx::query("SELECT 1").execute(pool).await?;
    tracing::debug!("database health check passed");
    Ok(())
}
