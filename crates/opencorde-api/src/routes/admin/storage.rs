//! # Admin Storage Handlers
//! Endpoints for viewing and updating object storage configuration, plus
//! connection testing and health checks.
//!
//! ## Endpoints
//! - GET  /api/v1/admin/storage — Return current storage config (secrets masked)
//! - PUT  /api/v1/admin/storage — Update storage config
//! - POST /api/v1/admin/storage/test — Test connection to the configured storage
//!
//! ## Depends On
//! - crate::middleware::auth::AuthUser
//! - crate::AppState
//! - aws_sdk_s3 for connection testing

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::time::{Duration, Instant};

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

use super::handlers::is_admin;

/// Supported storage providers.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StorageProvider {
    Minio,
    AwsS3,
    OtherS3,
}

impl std::fmt::Display for StorageProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageProvider::Minio => write!(f, "minio"),
            StorageProvider::AwsS3 => write!(f, "aws_s3"),
            StorageProvider::OtherS3 => write!(f, "other_s3"),
        }
    }
}

impl Default for StorageProvider {
    fn default() -> Self {
        StorageProvider::Minio
    }
}

/// Storage configuration as stored in the database.
#[derive(Debug, Deserialize, Serialize)]
pub struct StorageConfig {
    pub provider: StorageProvider,
    pub endpoint: String,
    pub access_key: Option<String>,
    /// Secret key is masked in GET responses.
    pub secret_key: Option<String>,
    pub bucket: String,
    pub region: String,
    pub force_path_style: bool,
    pub use_ssl: bool,
}

/// Response for GET /api/v1/admin/storage.
/// Secret key is always masked.
#[derive(Debug, Serialize)]
pub struct StorageConfigResponse {
    pub provider: String,
    pub endpoint: String,
    pub access_key: Option<String>,
    pub secret_key_masked: bool,
    pub bucket: String,
    pub region: String,
    pub force_path_style: bool,
    pub use_ssl: bool,
}

/// Request body for PUT /api/v1/admin/storage.
#[derive(Debug, Deserialize)]
pub struct UpdateStorageRequest {
    pub provider: String,
    pub endpoint: String,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    pub bucket: String,
    pub region: String,
    pub force_path_style: bool,
    pub use_ssl: bool,
}

/// Result of a storage connection test.
#[derive(Debug, Serialize)]
pub struct StorageTestResult {
    pub ok: bool,
    pub latency_ms: u128,
    pub error: Option<String>,
    pub bucket_exists: bool,
}

use super::types::StorageHealth;

// ------------------------------------------------------------------
// DB helpers
// ------------------------------------------------------------------

async fn get_storage_config_db(db: &sqlx::PgPool) -> Result<Option<StorageConfig>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT provider, endpoint, access_key, secret_key, bucket, region, force_path_style, use_ssl
         FROM storage_config WHERE id = TRUE",
    )
    .fetch_optional(db)
    .await?;

    Ok(row.map(|r| StorageConfig {
        provider: match r.get::<String, _>("provider").as_str() {
            "aws_s3" => StorageProvider::AwsS3,
            "other_s3" => StorageProvider::OtherS3,
            _ => StorageProvider::Minio,
        },
        endpoint: r.get("endpoint"),
        access_key: r.get("access_key"),
        secret_key: r.get("secret_key"),
        bucket: r.get("bucket"),
        region: r.get("region"),
        force_path_style: r.get("force_path_style"),
        use_ssl: r.get("use_ssl"),
    }))
}

async fn upsert_storage_config_db(
    db: &sqlx::PgPool,
    config: &StorageConfig,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO storage_config (
            id, provider, endpoint, access_key, secret_key, bucket, region, force_path_style, use_ssl
        ) VALUES (TRUE, $1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (id) DO UPDATE SET
            provider = EXCLUDED.provider,
            endpoint = EXCLUDED.endpoint,
            access_key = EXCLUDED.access_key,
            secret_key = EXCLUDED.secret_key,
            bucket = EXCLUDED.bucket,
            region = EXCLUDED.region,
            force_path_style = EXCLUDED.force_path_style,
            use_ssl = EXCLUDED.use_ssl",
    )
    .bind(config.provider.to_string())
    .bind(&config.endpoint)
    .bind(&config.access_key)
    .bind(&config.secret_key)
    .bind(&config.bucket)
    .bind(&config.region)
    .bind(config.force_path_style)
    .bind(config.use_ssl)
    .execute(db)
    .await?;

    Ok(())
}

// ------------------------------------------------------------------
// GET /api/v1/admin/storage
// ------------------------------------------------------------------

/// GET /api/v1/admin/storage — Return current storage configuration.
///
/// Requires admin role. Secret key is masked in the response.
#[tracing::instrument(skip(state, auth))]
pub async fn get_storage_config(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<StorageConfigResponse>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "admin: fetching storage config");

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted storage config read");
        return Err(ApiError::Forbidden);
    }

    let config = get_storage_config_db(&state.db).await.map_err(|e| {
        tracing::error!(error = %e, "admin: failed to load storage config");
        ApiError::InternalServerError(e.to_string())
    })?;

    // If no DB config exists yet, return the current env-based config as a fallback.
    let resp = match config {
        Some(c) => StorageConfigResponse {
            provider: c.provider.to_string(),
            endpoint: c.endpoint,
            access_key: c.access_key,
            secret_key_masked: c.secret_key.is_some(),
            bucket: c.bucket,
            region: c.region,
            force_path_style: c.force_path_style,
            use_ssl: c.use_ssl,
        },
        None => StorageConfigResponse {
            provider: "minio".to_string(),
            endpoint: state.config.minio_endpoint.clone(),
            access_key: Some(state.config.minio_access_key.clone()),
            secret_key_masked: true,
            bucket: state.config.minio_bucket.clone(),
            region: "us-east-1".to_string(),
            force_path_style: true,
            use_ssl: state.config.minio_endpoint.starts_with("https://"),
        },
    };

    tracing::info!(provider = %resp.provider, endpoint = %resp.endpoint, "admin: storage config fetched");
    Ok(Json(resp))
}

// ------------------------------------------------------------------
// PUT /api/v1/admin/storage
// ------------------------------------------------------------------

/// PUT /api/v1/admin/storage — Update storage configuration.
///
/// Requires admin role. Changes are persisted to the database.
/// The running S3 client is NOT rebuilt automatically — a restart is
/// required for changes to take effect on uploads.
#[tracing::instrument(skip(state, auth, body))]
pub async fn update_storage_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<UpdateStorageRequest>,
) -> Result<Json<StorageConfigResponse>, ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        provider = %body.provider,
        endpoint = %body.endpoint,
        bucket = %body.bucket,
        "admin: updating storage config"
    );

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted storage config update");
        return Err(ApiError::Forbidden);
    }

    if body.endpoint.is_empty() || body.endpoint.len() > 512 {
        return Err(ApiError::BadRequest(
            "endpoint must be between 1 and 512 characters".into(),
        ));
    }

    if body.bucket.is_empty() || body.bucket.len() > 128 {
        return Err(ApiError::BadRequest(
            "bucket must be between 1 and 128 characters".into(),
        ));
    }

    let provider = match body.provider.as_str() {
        "aws_s3" => StorageProvider::AwsS3,
        "other_s3" => StorageProvider::OtherS3,
        _ => StorageProvider::Minio,
    };

    let config = StorageConfig {
        provider,
        endpoint: body.endpoint,
        access_key: body.access_key,
        secret_key: body.secret_key,
        bucket: body.bucket,
        region: body.region,
        force_path_style: body.force_path_style,
        use_ssl: body.use_ssl,
    };

    upsert_storage_config_db(&state.db, &config)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "admin: failed to save storage config");
            ApiError::InternalServerError(e.to_string())
        })?;

    let resp = StorageConfigResponse {
        provider: config.provider.to_string(),
        endpoint: config.endpoint,
        access_key: config.access_key,
        secret_key_masked: config.secret_key.is_some(),
        bucket: config.bucket,
        region: config.region,
        force_path_style: config.force_path_style,
        use_ssl: config.use_ssl,
    };

    tracing::info!("admin: storage config updated");
    Ok(Json(resp))
}

// ------------------------------------------------------------------
// POST /api/v1/admin/storage/test
// ------------------------------------------------------------------

/// Build a temporary S3 client from the provided or stored config and test
/// connectivity by attempting to head the configured bucket.
async fn test_storage_connection_inner(config: &StorageConfig) -> StorageTestResult {
    let started = Instant::now();

    let creds = aws_sdk_s3::config::Credentials::new(
        config.access_key.as_deref().unwrap_or(""),
        config.secret_key.as_deref().unwrap_or(""),
        None,
        None,
        "opencorde-test",
    );

    let mut builder = aws_sdk_s3::Config::builder()
        .credentials_provider(creds)
        .region(aws_sdk_s3::config::Region::new(config.region.clone()))
        .behavior_version_latest();

    // Only set endpoint_url for non-AWS providers or when endpoint is explicitly set.
    if !config.endpoint.is_empty() && config.endpoint != "https://s3.amazonaws.com" {
        builder = builder.endpoint_url(&config.endpoint);
    }

    if config.force_path_style {
        builder = builder.force_path_style(true);
    }

    let s3_config = builder.build();
    let client = aws_sdk_s3::Client::from_conf(s3_config);

    // Try to head the bucket as a connectivity + permission test.
    match client.head_bucket().bucket(&config.bucket).send().await {
        Ok(_) => StorageTestResult {
            ok: true,
            latency_ms: started.elapsed().as_millis(),
            error: None,
            bucket_exists: true,
        },
        Err(e) => {
            let err_str = format!("{:#}", e);
            // Some providers return 404 for head_bucket even when credentials work;
            // treat 404 as "connected but bucket not found".
            let bucket_exists = !err_str.contains("404") && !err_str.contains("NoSuchBucket");
            let ok = err_str.contains("404")
                || err_str.contains("403")
                || err_str.contains("NoSuchBucket")
                || bucket_exists;

            StorageTestResult {
                ok,
                latency_ms: started.elapsed().as_millis(),
                error: Some(err_str),
                bucket_exists,
            }
        }
    }
}

/// POST /api/v1/admin/storage/test — Test storage connectivity.
///
/// Requires admin role. Accepts an optional config override in the body;
/// if omitted, tests the currently stored (or env fallback) config.
#[tracing::instrument(skip(state, auth))]
pub async fn test_storage_connection(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<Option<UpdateStorageRequest>>,
) -> Result<Json<StorageTestResult>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "admin: testing storage connection");

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted storage test");
        return Err(ApiError::Forbidden);
    }

    let config = match body {
        Some(req) => StorageConfig {
            provider: match req.provider.as_str() {
                "aws_s3" => StorageProvider::AwsS3,
                "other_s3" => StorageProvider::OtherS3,
                _ => StorageProvider::Minio,
            },
            endpoint: req.endpoint,
            access_key: req.access_key,
            secret_key: req.secret_key,
            bucket: req.bucket,
            region: req.region,
            force_path_style: req.force_path_style,
            use_ssl: req.use_ssl,
        },
        None => get_storage_config_db(&state.db)
            .await
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?
            .unwrap_or_else(|| StorageConfig {
                provider: StorageProvider::Minio,
                endpoint: state.config.minio_endpoint.clone(),
                access_key: Some(state.config.minio_access_key.clone()),
                secret_key: Some(state.config.minio_secret_key.clone()),
                bucket: state.config.minio_bucket.clone(),
                region: "us-east-1".to_string(),
                force_path_style: true,
                use_ssl: state.config.minio_endpoint.starts_with("https://"),
            }),
    };

    let result = test_storage_connection_inner(&config).await;

    tracing::info!(
        ok = result.ok,
        latency_ms = result.latency_ms,
        bucket_exists = result.bucket_exists,
        error = ?result.error,
        "admin: storage connection test completed"
    );

    Ok(Json(result))
}

// ------------------------------------------------------------------
// Storage health (used by admin stats)
// ------------------------------------------------------------------

/// Check storage health for the admin stats endpoint.
/// Uses the currently active S3 client from AppState to verify connectivity.
pub async fn get_storage_health(state: &AppState) -> StorageHealth {
    let started = Instant::now();

    // Try to list objects in the bucket with max-keys=1 as a lightweight check.
    match state
        .s3
        .list_objects_v2()
        .bucket(&state.config.minio_bucket)
        .max_keys(1)
        .send()
        .await
    {
        Ok(_) => StorageHealth {
            ok: true,
            latency_ms: Some(started.elapsed().as_millis()),
            error: None,
            bucket: state.config.minio_bucket.clone(),
        },
        Err(e) => StorageHealth {
            ok: false,
            latency_ms: Some(started.elapsed().as_millis()),
            error: Some(format!("{:#}", e)),
            bucket: state.config.minio_bucket.clone(),
        },
    }
}
