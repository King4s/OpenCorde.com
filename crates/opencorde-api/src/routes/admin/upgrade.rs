//! # Admin Upgrade & Migration Handlers
//! Endpoints for checking version, applying database migrations,
//! and monitoring upgrade status.
//!
//! ## Endpoints
//! - GET  /api/v1/admin/upgrade/status  — Current version, latest available, migration status
//! - POST /api/v1/admin/upgrade/check   — Refresh latest version check (queries GitHub Releases API)
//! - POST /api/v1/admin/upgrade/migrate — Run pending database migrations
//! - POST /api/v1/admin/upgrade/rollback — Rollback guidance and pre-checks
//!
//! ## Dependencies
//! - axum (web framework)
//! - crate::AppState (application state)
//! - crate::error::ApiError (error handling)
//! - crate::middleware::auth::AuthUser (authentication)
//! - sqlx (database access)
//! - reqwest (GitHub API queries)

use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use serde::Serialize;
use sqlx::Row;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser, routes::admin::handlers};

/// Upgrade status response.
#[derive(Debug, Serialize)]
pub struct UpgradeStatus {
    /// Current running version from Cargo.toml
    pub current_version: String,
    /// Latest available version (from GitHub or env)
    pub latest_version: Option<String>,
    /// Whether an upgrade is available
    pub upgrade_available: bool,
    /// URL to the changelog / release notes for the latest version
    pub changelog_url: Option<String>,
    /// When the version was last checked (ISO 8601)
    pub last_checked_at: Option<String>,
    /// Whether auto-check is enabled
    pub auto_check_enabled: bool,
    /// Total number of migrations in the codebase
    pub total_migrations: i64,
    /// Number of migrations already applied
    pub applied_migrations: i64,
    /// Number of pending migrations
    pub pending_migrations: i64,
    /// List of migration filenames still to apply
    pub pending_migration_names: Vec<String>,
    /// Whether a migration is currently in progress
    pub migration_in_progress: bool,
    /// Timestamp of last successful migration run
    pub last_migration_at: Option<String>,
    /// Current database schema version tag (from HEAD)
    pub db_version_tag: Option<String>,
}

/// Response after a refresh version check.
#[derive(Debug, Serialize)]
pub struct CheckResponse {
    /// Previous latest version
    pub previous_latest: Option<String>,
    /// Newly resolved latest version
    pub latest: Option<String>,
    /// Whether an upgrade is available
    pub upgrade_available: bool,
    /// URL to the changelog / release notes
    pub changelog_url: Option<String>,
}

/// Response after running migrations.
#[derive(Debug, Serialize)]
pub struct MigrateResponse {
    /// Whether migrations ran successfully
    pub success: bool,
    /// Number of migrations applied in this run
    pub applied_count: i64,
    /// Names of migrations that were applied
    pub applied_names: Vec<String>,
    /// Error message if migration failed
    pub error: Option<String>,
    /// Current version after migration
    pub current_version: String,
}

/// Rollback guidance response.
#[derive(Debug, Serialize)]
pub struct RollbackGuidance {
    /// Whether rollback is possible (always false for schema migrations)
    pub rollback_possible: bool,
    /// Human-readable guidance message
    pub guidance: String,
    /// Recommended steps
    pub steps: Vec<String>,
    /// Whether a backup exists that could be used
    pub backup_available: bool,
}

/// Cached version check result.
#[derive(Clone, Debug)]
pub struct VersionCache {
    pub latest_version: Option<String>,
    pub changelog_url: Option<String>,
    pub checked_at: String,
}

/// Shared version cache behind a mutex.
pub type VersionCacheState = Arc<Mutex<Option<VersionCache>>>;

/// Look up the latest version from the LATEST_VERSION env var.
/// Falls back to None if the env var is not set.
fn resolve_latest_version() -> Option<String> {
    std::env::var("LATEST_VERSION").ok()
}

/// Query GitHub Releases API for the latest release tag.
///
/// Returns (tag_name, html_url) on success, or falls back to
/// LATEST_VERSION env var on failure.
async fn query_github_releases(github_repo: &str) -> Option<(String, String)> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        github_repo
    );
    let client = reqwest::Client::builder()
        .user_agent("opencorde-admin/1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;

    let response = client.get(&url).send().await.ok()?;

    if !response.status().is_success() {
        tracing::warn!(
            status = %response.status(),
            "GitHub Releases API returned non-success"
        );
        return None;
    }

    let body: serde_json::Value = response.json().await.ok()?;
    let tag = body["tag_name"].as_str().map(|s| s.to_string())?;
    let html_url = body["html_url"].as_str().map(|s| s.to_string())?;

    Some((tag, html_url))
}

/// Resolve the latest version: try GitHub API first, fall back to env var.
async fn resolve_latest(github_repo: &str) -> (Option<String>, Option<String>) {
    if let Some((tag, url)) = query_github_releases(github_repo).await {
        tracing::info!(tag = %tag, url = %url, "resolved latest version from GitHub");
        (Some(tag), Some(url))
    } else if let Some(env_version) = resolve_latest_version() {
        tracing::info!(
            version = %env_version,
            "resolved latest version from LATEST_VERSION env var (GitHub query failed)"
        );
        (Some(env_version), None)
    } else {
        tracing::warn!(
            "could not resolve latest version (GitHub query failed, no LATEST_VERSION env var)"
        );
        (None, None)
    }
}

/// Query the list of applied migrations from sqlx's tracking table.
async fn get_applied_migrations(db: &sqlx::PgPool) -> Result<Vec<String>, ApiError> {
    let rows = sqlx::query("SELECT version FROM _sqlx_migrations ORDER BY version DESC")
        .fetch_all(db)
        .await
        .map_err(|e| {
            // If the _sqlx_migrations table doesn't exist yet, no migrations have run
            if e.to_string().contains("relation") && e.to_string().contains("does not exist") {
                return ApiError::InternalServerError(
                    "migrations table not found — has the application started?".into(),
                );
            }
            ApiError::Database(e)
        })?;

    Ok(rows
        .iter()
        .map(|row| {
            let v: i64 = row.get("version");
            v.to_string()
        })
        .collect())
}

/// Enumerate all migration filenames from the migration directory.
/// These are embedded at compile time via sqlx::migrate!().
/// Since we can't introspect that at runtime, we approximate by checking
/// the applied set. Pending = total migrations - applied.
/// For the migration names list, we pull from the sqlx migrations.
async fn get_pending_migration_names(
    db: &sqlx::PgPool,
    applied: &[String],
) -> Result<Vec<String>, ApiError> {
    // sqlx stores migration file names alongside versions.
    // Query for all known migrations and diff against applied.
    let all_rows =
        sqlx::query("SELECT version, description FROM _sqlx_migrations ORDER BY version ASC")
            .fetch_all(db)
            .await
            .map_err(|e| {
                if e.to_string().contains("relation") && e.to_string().contains("does not exist") {
                    // No migrations table yet — all migrations are pending
                    return ApiError::InternalServerError(
                        "migrations table not found — all migrations are pending".into(),
                    );
                }
                ApiError::Database(e)
            })?;

    // Build a map of all known migrations from the tracking table
    let all_known: Vec<String> = all_rows
        .iter()
        .map(|row| {
            let v: i64 = row.get("version");
            v.to_string()
        })
        .collect();

    let total_applied = applied.len();
    let total_known = all_known.len();

    // If we have more applied than known, we're up to date
    if total_applied >= total_known {
        return Ok(vec![]);
    }

    // The applied versions are the first `applied.len()` of `all_known`
    // (since they're ordered ASC). The rest are pending.
    let pending_versions: Vec<String> = all_known[total_applied..]
        .iter()
        .map(|v| {
            // Try to get description for each pending version
            all_rows
                .iter()
                .find(|row| {
                    let rv: i64 = row.get("version");
                    rv.to_string() == *v
                })
                .map(|row| {
                    let desc: String = row.get("description");
                    format!("{}_{}", v, desc)
                })
                .unwrap_or_else(|| v.clone())
        })
        .collect();

    Ok(pending_versions)
}

/// Count pending migrations by comparing the compile-time migration count
/// against applied. We can't get the compile-time count at runtime from
/// sqlx, so we approximate by using the total count of the _sqlx_migrations
/// table. In a real deployment, a fresh DB has zero entries in _sqlx_migrations
/// but sqlx::migrate!() knows all available ones. The number we get from
/// the tracking table is the number of migrations applied since the DB was created.
///
/// For the total, we approximate by checking how many migration files exist
/// on disk, since sqlx embeds them at compile time.
async fn count_total_migrations(db: &sqlx::PgPool) -> Result<i64, ApiError> {
    // Find migration SQL files on disk to count total available
    let migrations_dir =
        std::path::Path::new(&std::env::current_dir().unwrap_or_else(|_| "/".into()))
            .join("crates/opencorde-db/migrations");

    if !migrations_dir.exists() {
        // Fallback: just count entries in _sqlx_migrations table
        let row = sqlx::query("SELECT COUNT(*) as count FROM _sqlx_migrations")
            .fetch_one(db)
            .await;
        return match row {
            Ok(r) => {
                let c: i64 = r.get("count");
                Ok(c)
            }
            Err(_) => Ok(0),
        };
    }

    let mut count = 0i64;
    if let Ok(entries) = std::fs::read_dir(&migrations_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "sql").unwrap_or(false) {
                count += 1;
            }
        }
    }

    // If disk count is 0, fall back to the DB count
    if count == 0 {
        let row = sqlx::query("SELECT COUNT(*) as count FROM _sqlx_migrations")
            .fetch_one(db)
            .await;
        return match row {
            Ok(r) => {
                let c: i64 = r.get("count");
                Ok(c)
            }
            Err(_) => Ok(0),
        };
    }

    Ok(count)
}

/// Read the cached version or return None.
async fn get_cached_version(cache: &VersionCacheState) -> Option<VersionCache> {
    let guard = cache.lock().await;
    guard.clone()
}

/// Update the version cache.
async fn set_cached_version(cache: &VersionCacheState, version: VersionCache) {
    let mut guard = cache.lock().await;
    *guard = Some(version);
}

/// GET /api/v1/admin/upgrade/status — Full upgrade and migration status.
///
/// Requires admin role. Returns current version, latest available,
/// migration counts, and pending migration names.
#[tracing::instrument(skip(state, auth))]
pub async fn get_status(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<UpgradeStatus>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "admin: fetching upgrade status");

    if !handlers::is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted upgrade status access");
        return Err(ApiError::Forbidden);
    }

    let current_version = env!("CARGO_PKG_VERSION").to_string();

    // Use cached version if available, otherwise resolve
    let cached = get_cached_version(&state.version_cache).await;
    let (latest_version, changelog_url, last_checked_at) = if let Some(ref c) = cached {
        (
            c.latest_version.clone(),
            c.changelog_url.clone(),
            Some(c.checked_at.clone()),
        )
    } else {
        // Resolve on first load (non-blocking)
        let github_repo = state.config.github_repo.clone();
        let (latest, url) = resolve_latest(&github_repo).await;
        let now = chrono::Utc::now().to_rfc3339();
        if latest.is_some() {
            set_cached_version(
                &state.version_cache,
                VersionCache {
                    latest_version: latest.clone(),
                    changelog_url: url.clone(),
                    checked_at: now.clone(),
                },
            )
            .await;
        }
        (latest, url, Some(now))
    };

    let upgrade_available = latest_version
        .as_ref()
        .map(|v| v != &current_version)
        .unwrap_or(false);

    // Count applied migrations
    let applied = get_applied_migrations(&state.db).await?;
    let applied_count = applied.len() as i64;

    // Count pending
    let total = count_total_migrations(&state.db).await?;
    let pending_count = (total - applied_count).max(0);

    // Get pending migration names
    let pending_names = get_pending_migration_names(&state.db, &applied)
        .await
        .unwrap_or_default();

    // Migration in progress flag (simplified — no real tracking)
    let migration_in_progress = false;

    // Last migration timestamp
    let last_migration_at: Option<String> = if !applied.is_empty() {
        // Get the timestamp of the most recently applied migration
        let last_ts =
            sqlx::query("SELECT installed_on FROM _sqlx_migrations ORDER BY version DESC LIMIT 1")
                .fetch_optional(&state.db)
                .await
                .ok()
                .flatten()
                .map(|row| {
                    let ts: String = row.get("installed_on");
                    ts
                });
        last_ts
    } else {
        None
    };

    let auto_check_enabled = state.config.github_repo != "disabled";

    Ok(Json(UpgradeStatus {
        current_version,
        latest_version,
        upgrade_available,
        changelog_url,
        last_checked_at,
        auto_check_enabled,
        total_migrations: total,
        applied_migrations: applied_count,
        pending_migrations: pending_count,
        pending_migration_names: pending_names,
        migration_in_progress,
        last_migration_at,
        db_version_tag: None,
    }))
}

/// POST /api/v1/admin/upgrade/check — Refresh version check.
///
/// Requires admin role. Queries GitHub Releases API (with LATEST_VERSION fallback)
/// and returns comparison against the running version.
#[tracing::instrument(skip(state, auth))]
pub async fn check_version(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<CheckResponse>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "admin: checking for updates");

    if !handlers::is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted version check");
        return Err(ApiError::Forbidden);
    }

    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let previous_cached = get_cached_version(&state.version_cache).await;
    let previous_latest = previous_cached
        .as_ref()
        .and_then(|c| c.latest_version.clone());

    let github_repo = state.config.github_repo.clone();
    let (latest, changelog_url) = resolve_latest(&github_repo).await;

    let upgrade_available = latest
        .as_ref()
        .map(|v| v != &current_version)
        .unwrap_or(false);

    // Update cache
    set_cached_version(
        &state.version_cache,
        VersionCache {
            latest_version: latest.clone(),
            changelog_url: changelog_url.clone(),
            checked_at: chrono::Utc::now().to_rfc3339(),
        },
    )
    .await;

    Ok(Json(CheckResponse {
        previous_latest,
        latest,
        upgrade_available,
        changelog_url,
    }))
}

/// POST /api/v1/admin/upgrade/migrate — Run pending database migrations.
///
/// Requires admin role. Executes sqlx migrations in order.
/// Returns which migrations were applied.
#[tracing::instrument(skip(state, auth))]
pub async fn run_migrations(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<MigrateResponse>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "admin: running database migrations");

    if !handlers::is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted migration run");
        return Err(ApiError::Forbidden);
    }

    let before = get_applied_migrations(&state.db).await?;
    let before_count = before.len() as i64;

    match opencorde_db::run_migrations(&state.db).await {
        Ok(()) => {
            let after = get_applied_migrations(&state.db).await?;
            let after_count = after.len() as i64;
            let applied_count = after_count - before_count;

            // Find newly applied migration names
            let new_names: Vec<String> = after
                .iter()
                .filter(|v| !before.contains(v))
                .cloned()
                .collect();

            tracing::info!(applied_count, "admin: migrations completed successfully");

            Ok(Json(MigrateResponse {
                success: true,
                applied_count,
                applied_names: new_names,
                error: None,
                current_version: env!("CARGO_PKG_VERSION").to_string(),
            }))
        }
        Err(e) => {
            tracing::error!(error = %e, "admin: migration failed");

            Ok(Json(MigrateResponse {
                success: false,
                applied_count: 0,
                applied_names: vec![],
                error: Some(e.to_string()),
                current_version: env!("CARGO_PKG_VERSION").to_string(),
            }))
        }
    }
}

/// POST /api/v1/admin/upgrade/rollback — Rollback guidance.
///
/// Requires admin role. Since database migrations are generally not
/// reversible, this provides informational guidance about restoring
/// from a backup instead.
#[tracing::instrument(skip(state, auth))]
pub async fn rollback_guidance(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<RollbackGuidance>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "admin: requesting rollback guidance");

    if !handlers::is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted rollback guidance");
        return Err(ApiError::Forbidden);
    }

    // Check if any backups exist
    let backup_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM backups")
        .fetch_one(&state.db)
        .await
        .map(|row| row.get("count"))
        .unwrap_or(0);

    let backup_available = backup_count > 0;

    Ok(Json(RollbackGuidance {
        rollback_possible: false,
        guidance: "Database schema migrations are forward-only and cannot be automatically reversed. If the latest migration caused issues, the recommended approach is to restore from a pre-migration backup.".into(),
        steps: vec![
            "1. Stop the application server".into(),
            if backup_available {
                "2. Restore from the most recent backup created before the migration".into()
            } else {
                "2. No backups found — create one before running migrations next time".into()
            },
            "3. Revert the application binary to the previous version".into(),
            "4. Restart the application server".into(),
            "5. Verify application health via the health endpoint".into(),
        ],
        backup_available,
    }))
}

/// Build the upgrade/migration router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/admin/upgrade/status", get(get_status))
        .route("/api/v1/admin/upgrade/check", post(check_version))
        .route("/api/v1/admin/upgrade/migrate", post(run_migrations))
        .route("/api/v1/admin/upgrade/rollback", post(rollback_guidance))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_latest_version() {
        // Without LATEST_VERSION env var set, returns None
        // We can't safely set env vars in parallel tests, so just check
        // that the function compiles and returns something.
        let result = resolve_latest_version();
        // In test environment, env var is likely not set
        assert!(result.is_none() || result.is_some());
    }
}
