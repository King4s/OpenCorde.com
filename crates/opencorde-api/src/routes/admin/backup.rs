//! # Admin Backup Handlers
//! Endpoints for creating, listing, downloading, and restoring instance backups.
//!
//! ## Endpoints
//! - POST /api/v1/admin/backups — Trigger a manual backup
//! - GET  /api/v1/admin/backups — List existing backups
//! - GET  /api/v1/admin/backups/{id}/download — Download a backup archive
//! - POST /api/v1/admin/backups/{id}/restore — Restore from a backup
//!
//! ## Backup Contents
//! Each backup is a `.tar.gz` archive containing:
//! - `dump.sql` — PostgreSQL database dump via `pg_dump`
//! - `files/` — Downloaded S3/MinIO objects keyed by `bucket_key`
//! - `manifest.json` — Metadata (created_at, file count, sizes)
//!
//! ## Depends On
//! - `tokio::process::Command` for `pg_dump` / `psql`
//! - `aws_sdk_s3` for object listing and download
//! - `tar` + `flate2` for archive creation

use axum::{
    Json, Router,
    extract::{Path, State},
    http::header,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    io::Write,
    path::{Path as StdPath, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::process::Command;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};
use super::handlers::is_admin;
use super::types::{BackupInfo, RestoreRequest};

const BACKUP_DIR: &str = "backups";

// ------------------------------------------------------------------
// Router
// ------------------------------------------------------------------

/// Build the backup router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/admin/backups", post(create_backup))
        .route("/api/v1/admin/backups", get(list_backups))
        .route("/api/v1/admin/backups/{id}/download", get(download_backup))
        .route("/api/v1/admin/backups/{id}/restore", post(restore_backup))
}

// ------------------------------------------------------------------
// POST /api/v1/admin/backups
// ------------------------------------------------------------------

/// Create a new instance backup.
///
/// 1. Runs `pg_dump` using the configured `DATABASE_URL`.
/// 2. Queries the `files` table and downloads every S3 object into `files/`.
/// 3. Writes a `manifest.json` with metadata.
/// 4. Archives the working directory into a `.tar.gz`.
/// 5. Cleans up the temporary working directory.
#[tracing::instrument(skip(state, auth))]
async fn create_backup(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<BackupInfo>, ApiError> {
    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted backup creation");
        return Err(ApiError::Forbidden);
    }

    let backup_id = generate_backup_id();
    tracing::info!(backup_id = %backup_id, "admin: starting backup");

    let base_dir = PathBuf::from(BACKUP_DIR);
    let work_dir = base_dir.join(&backup_id);
    let archive_path = base_dir.join(format!("{}.tar.gz", backup_id));

    tokio::fs::create_dir_all(&work_dir)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "admin: failed to create backup work directory");
            ApiError::Internal(anyhow::anyhow!("failed to create backup directory: {}", e))
        })?;

    // 1. Database dump
    let dump_path = work_dir.join("dump.sql");
    let db_size = dump_database(&state.config.database_url, &dump_path)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "admin: pg_dump failed");
            ApiError::Internal(anyhow::anyhow!("pg_dump failed: {}", e))
        })?;

    // 2. Download S3 files
    let files_dir = work_dir.join("files");
    tokio::fs::create_dir_all(&files_dir)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "admin: failed to create files directory");
            ApiError::Internal(anyhow::anyhow!("failed to create files directory: {}", e))
        })?;

    let file_count = download_all_s3_files(&state, &files_dir)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "admin: S3 file download failed");
            ApiError::Internal(anyhow::anyhow!("S3 download failed: {}", e))
        })?;

    // 3. Write manifest
    let manifest = BackupManifest {
        id: backup_id.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        db_size_bytes: db_size,
        file_count,
        database_url: mask_secret(&state.config.database_url),
    };
    let manifest_path = work_dir.join("manifest.json");
    tokio::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "admin: failed to write manifest");
        ApiError::Internal(anyhow::anyhow!("manifest write failed: {}", e))
    })?;

    // 4. Create tar.gz (CPU-bound → spawn_blocking)
    let work_dir_clone = work_dir.clone();
    let archive_path_clone = archive_path.clone();
    tokio::task::spawn_blocking(move || create_tar_gz(&work_dir_clone, &archive_path_clone))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "admin: tar.gz task panicked");
            ApiError::Internal(anyhow::anyhow!("archive creation panicked: {}", e))
        })?
        .map_err(|e| {
            tracing::error!(error = %e, "admin: tar.gz creation failed");
            ApiError::Internal(anyhow::anyhow!("archive creation failed: {}", e))
        })?;

    // 5. Clean up working directory
    if let Err(e) = tokio::fs::remove_dir_all(&work_dir).await {
        tracing::warn!(error = %e, "admin: failed to clean up backup work directory");
    }

    // 6. Get final archive size
    let archive_size = tokio::fs::metadata(&archive_path)
        .await
        .map(|m| m.len())
        .unwrap_or(0);

    tracing::info!(
        backup_id = %backup_id,
        db_size = db_size,
        file_count = file_count,
        archive_size = archive_size,
        "admin: backup completed"
    );

    Ok(Json(BackupInfo {
        id: backup_id,
        created_at: manifest.created_at,
        size_bytes: archive_size,
        db_size_bytes: db_size,
        file_count,
        status: "completed".to_string(),
    }))
}

// ------------------------------------------------------------------
// GET /api/v1/admin/backups
// ------------------------------------------------------------------

/// List all existing backups from the `backups/` directory.
#[tracing::instrument(skip(state, auth))]
async fn list_backups(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<BackupInfo>>, ApiError> {
    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted backup listing");
        return Err(ApiError::Forbidden);
    }

    let mut backups = Vec::new();
    let base_dir = PathBuf::from(BACKUP_DIR);

    if let Ok(mut entries) = tokio::fs::read_dir(&base_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if !name_str.ends_with(".tar.gz") {
                continue;
            }
            let id = name_str.trim_end_matches(".tar.gz").to_string();
            let meta = entry.metadata().await.ok();
            let size_bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let created_at = meta
                .and_then(|m| m.created().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| {
                    chrono::DateTime::from_timestamp(d.as_secs() as i64, 0)
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_default()
                })
                .unwrap_or_default();

            backups.push(BackupInfo {
                id,
                created_at,
                size_bytes,
                db_size_bytes: 0, // Could extract from manifest if needed
                file_count: 0,
                status: "completed".to_string(),
            });
        }
    }

    // Sort newest first
    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(Json(backups))
}

// ------------------------------------------------------------------
// GET /api/v1/admin/backups/{id}/download
// ------------------------------------------------------------------

/// Download a backup archive as an attachment.
#[tracing::instrument(skip(state, auth))]
async fn download_backup(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted backup download");
        return Err(ApiError::Forbidden);
    }

    let path = PathBuf::from(BACKUP_DIR).join(format!("{}.tar.gz", id));
    if !path.exists() {
        return Err(ApiError::NotFound(format!("backup '{}' not found", id)));
    }

    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, backup_id = %id, "admin: failed to read backup archive");
            ApiError::Internal(anyhow::anyhow!("failed to read backup: {}", e))
        })?;

    let filename = format!("{}.tar.gz", id);
    let headers = [
        (header::CONTENT_TYPE, "application/gzip".to_string()),
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        ),
    ];

    tracing::info!(backup_id = %id, size = bytes.len(), "admin: backup downloaded");
    Ok((headers, bytes))
}

// ------------------------------------------------------------------
// POST /api/v1/admin/backups/{id}/restore
// ------------------------------------------------------------------

/// Restore the database from a backup archive.
///
/// **DANGER**: This drops and recreates the public schema, then restores
/// from the backup's `dump.sql`. The request body must contain
/// `{ "confirm": "<backup_id>" }` as a safety measure.
///
/// Files are NOT automatically restored to S3 — only the database is restored.
/// S3 files must be re-uploaded separately if they are missing.
#[tracing::instrument(skip(state, auth, body))]
async fn restore_backup(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<RestoreRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted restore");
        return Err(ApiError::Forbidden);
    }

    if body.confirm != id {
        return Err(ApiError::BadRequest(
            "confirmation mismatch: pass the backup id as 'confirm' to proceed".into(),
        ));
    }

    let archive_path = PathBuf::from(BACKUP_DIR).join(format!("{}.tar.gz", id));
    if !archive_path.exists() {
        return Err(ApiError::NotFound(format!("backup '{}' not found", id)));
    }

    tracing::warn!(
        backup_id = %id,
        user_id = %auth.user_id,
        "admin: RESTORING DATABASE FROM BACKUP — THIS IS DESTRUCTIVE"
    );

    // Extract to a temporary directory
    let temp_dir = PathBuf::from(BACKUP_DIR).join(format!("restore_{}", id));
    if let Err(e) = tokio::fs::create_dir_all(&temp_dir).await {
        tracing::error!(error = %e, "admin: failed to create restore temp directory");
        return Err(ApiError::Internal(anyhow::anyhow!(
            "restore temp dir failed: {}",
            e
        )));
    }

    let archive_path_clone = archive_path.clone();
    let temp_dir_clone = temp_dir.clone();
    let extract_result = tokio::task::spawn_blocking(move || {
        extract_tar_gz(&archive_path_clone, &temp_dir_clone)
    })
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "admin: tar extract task panicked");
        ApiError::Internal(anyhow::anyhow!("extract panicked: {}", e))
    })?;

    if let Err(e) = extract_result {
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
        tracing::error!(error = %e, "admin: tar extraction failed");
        return Err(ApiError::Internal(anyhow::anyhow!("extract failed: {}", e)));
    }

    let dump_path = temp_dir.join(&id).join("dump.sql");
    if !dump_path.exists() {
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
        return Err(ApiError::BadRequest(
            "backup archive does not contain a database dump".into(),
        ));
    }

    // Restore via psql
    let db_url = state.config.database_url.clone();
    let restore_result = restore_database(&db_url, &dump_path).await;

    // Always clean up temp dir
    if let Err(e) = tokio::fs::remove_dir_all(&temp_dir).await {
        tracing::warn!(error = %e, "admin: failed to clean up restore temp directory");
    }

    restore_result.map_err(|e| {
        tracing::error!(error = %e, "admin: database restore failed");
        ApiError::Internal(anyhow::anyhow!("database restore failed: {}", e))
    })?;

    tracing::info!(backup_id = %id, "admin: database restore completed successfully");

    Ok(Json(json!({
        "success": true,
        "message": "Database restored successfully. S3 files were not modified; re-upload missing files if needed.",
    })))
}

// ------------------------------------------------------------------
// Helpers
// ------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct BackupManifest {
    id: String,
    created_at: String,
    db_size_bytes: u64,
    file_count: u64,
    database_url: String,
}

fn generate_backup_id() -> String {
    let now = chrono::Utc::now();
    format!("backup_{}", now.format("%Y%m%d_%H%M%S"))
}

fn mask_secret(secret: &str) -> String {
    if secret.len() > 6 {
        format!("{}...{}", &secret[..3], &secret[secret.len() - 3..])
    } else {
        "***".to_string()
    }
}

/// Run `pg_dump` and return the size of the resulting file.
async fn dump_database(db_url: &str, out_path: &PathBuf) -> anyhow::Result<u64> {
    let output = Command::new("pg_dump")
        .arg(db_url)
        .arg("--clean")
        .arg("--if-exists")
        .arg("--no-owner")
        .arg("--no-privileges")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("failed to spawn pg_dump: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("pg_dump failed: {}", stderr);
    }

    tokio::fs::write(out_path, &output.stdout)
        .await
        .map_err(|e| anyhow::anyhow!("failed to write dump file: {}", e))?;

    Ok(output.stdout.len() as u64)
}

/// Download every S3 object tracked in the `files` table.
async fn download_all_s3_files(state: &AppState, dest_dir: &PathBuf) -> anyhow::Result<u64> {
    let rows = sqlx::query("SELECT bucket_key FROM files")
        .fetch_all(&state.db)
        .await
        .map_err(|e| anyhow::anyhow!("failed to list files from database: {}", e))?;

    let mut count = 0u64;
    for row in rows {
        let key: String = row.try_get("bucket_key")
            .map_err(|e| anyhow::anyhow!("bucket_key column missing: {}", e))?;
        let safe_path = sanitize_path(&key);
        let file_path = dest_dir.join(&safe_path);

        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }

        let resp = state
            .s3
            .get_object()
            .bucket(&state.config.minio_bucket)
            .key(&key)
            .send()
            .await;

        match resp {
            Ok(obj) => {
                let body = obj
                    .body
                    .collect()
                    .await
                    .map_err(|e| anyhow::anyhow!("failed to collect S3 body for {}: {}", key, e))?;
                tokio::fs::write(&file_path, body.into_bytes())
                    .await
                    .map_err(|e| anyhow::anyhow!("failed to write file {}: {}", file_path.display(), e))?;
                count += 1;
            }
            Err(e) => {
                tracing::warn!(bucket_key = %key, error = %e, "admin: failed to download S3 object, skipping");
            }
        }
    }

    Ok(count)
}

/// Replace path separators with safe alternatives to avoid directory traversal.
fn sanitize_path(key: &str) -> String {
    key.replace('/', "__")
}

/// Create a tar.gz archive from a source directory.
fn create_tar_gz(src_dir: &PathBuf, out_path: &PathBuf) -> anyhow::Result<()> {
    let tar_gz = std::fs::File::create(out_path)?;
    let enc = flate2::write::GzEncoder::new(tar_gz, flate2::Compression::default());
    let mut tar = tar::Builder::new(enc);
    tar.append_dir_all(
        src_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .as_ref(),
        src_dir,
    )?;
    tar.into_inner()?.finish()?;
    Ok(())
}

/// Extract a tar.gz archive into a destination directory.
fn extract_tar_gz(archive_path: &PathBuf, dest_dir: &PathBuf) -> anyhow::Result<()> {
    let tar_gz = std::fs::File::open(archive_path)?;
    let dec = flate2::read::GzDecoder::new(tar_gz);
    let mut tar = tar::Archive::new(dec);
    tar.unpack(dest_dir)?;
    Ok(())
}

/// Restore a database dump using `psql`.
async fn restore_database(db_url: &str, dump_path: &PathBuf) -> anyhow::Result<()> {
    let dump_bytes = tokio::fs::read(dump_path)
        .await
        .map_err(|e| anyhow::anyhow!("failed to read dump file: {}", e))?;

    let mut child = Command::new("psql")
        .arg(db_url)
        .arg("--set")
        .arg("ON_ERROR_STOP=1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to spawn psql: {}", e))?;

    if let Some(stdin) = child.stdin.take() {
        let mut stdin = stdin;
        tokio::io::AsyncWriteExt::write_all(&mut stdin, &dump_bytes)
            .await
            .map_err(|e| anyhow::anyhow!("failed to write to psql stdin: {}", e))?;
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| anyhow::anyhow!("psql process error: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("psql restore failed: {}", stderr);
    }

    Ok(())
}
