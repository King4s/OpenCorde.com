//! # Route: File Uploads
//! Handles multipart file uploads to MinIO object storage.
//!
//! ## Endpoints
//! - POST /api/v1/channels/{channel_id}/attachments — Upload file attachment
//!
//! ## Response Types
//! - AttachmentResponse — File metadata and download URL
//!
//! ## Depends On
//! - axum::extract::{Multipart, State, Path}
//! - aws_sdk_s3 (MinIO storage)
//! - opencorde_db (files table insert)
//! - crate::middleware::auth::AuthUser
//! - crate::error::ApiError

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};
use crate::routes::helpers::parse_snowflake;

const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100 MB

/// Response body for successful file upload.
#[derive(Debug, Serialize, Deserialize)]
pub struct AttachmentResponse {
    /// Snowflake ID of the file record
    pub id: String,
    /// Original filename
    pub filename: String,
    /// MIME type (e.g., "application/pdf")
    pub content_type: String,
    /// File size in bytes
    pub size: i64,
    /// Download URL from MinIO
    pub url: String,
}

/// Build the uploads router with all endpoints.
pub fn router() -> Router<AppState> {
    Router::new().route(
        "/api/v1/channels/{channel_id}/attachments",
        post(upload_attachment),
    )
}

/// POST /api/v1/channels/{channel_id}/attachments — Upload a file attachment.
///
/// Accepts multipart/form-data with a single "file" field.
/// Stores the file in MinIO and records metadata in the database.
/// Requires authentication.
///
/// Returns 201 Created with attachment metadata.
#[instrument(skip(state, auth, multipart), fields(user_id = %auth.user_id))]
async fn upload_attachment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
    multipart: Multipart,
) -> Result<(StatusCode, Json<AttachmentResponse>), ApiError> {
    tracing::info!(channel_id = %channel_id, "uploading attachment");

    // Parse channel ID
    let _channel_id_sf = parse_snowflake(&channel_id)?;

    // Process multipart form
    let (filename, content_type, bytes) = extract_file_from_multipart(multipart).await?;

    tracing::debug!(
        filename = %filename,
        content_type = %content_type,
        size = bytes.len(),
        "file extracted from multipart"
    );

    // Validate file size
    let size = bytes.len() as u64;
    if size > MAX_FILE_SIZE {
        tracing::warn!(
            size = size,
            max = MAX_FILE_SIZE,
            "file exceeds maximum size"
        );
        return Err(ApiError::BadRequest(format!(
            "file size exceeds maximum of {} bytes",
            MAX_FILE_SIZE
        )));
    }

    // Generate S3 object key with UUID
    let object_uuid = Uuid::new_v4();
    let object_key = format!("attachments/{}/{}", object_uuid, filename);

    tracing::debug!(object_key = %object_key, "generated S3 object key");

    // Upload to MinIO
    state
        .s3
        .put_object()
        .bucket(&state.config.minio_bucket)
        .key(&object_key)
        .body(bytes.into())
        .content_type(&content_type)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "S3 upload failed");
            ApiError::Internal(anyhow::anyhow!("failed to upload file to S3: {}", e))
        })?;

    tracing::debug!(bucket = %state.config.minio_bucket, "file uploaded to S3");

    // Generate Snowflake ID for database record
    let mut generator = opencorde_core::snowflake::SnowflakeGenerator::new(1, 1);
    let file_id = generator.next_id();

    tracing::debug!(file_id = file_id.as_i64(), "generated Snowflake ID");

    // Insert file record into database
    sqlx::query_as::<_, (i64,)>(
        "INSERT INTO files (id, uploader_id, filename, bucket_key, size, content_type, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, NOW()) \
         RETURNING id",
    )
    .bind(file_id.as_i64())
    .bind(auth.user_id.as_i64())
    .bind(&filename)
    .bind(&object_key)
    .bind(size as i64)
    .bind(&content_type)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "database insert failed");
        ApiError::Database(e)
    })?;

    tracing::info!(
        file_id = file_id.as_i64(),
        object_key = %object_key,
        "file record inserted"
    );

    // Build public download URL
    let url = format!(
        "{}/{}/{}",
        state.config.files_public_url, state.config.minio_bucket, object_key
    );

    let response = AttachmentResponse {
        id: file_id.as_i64().to_string(),
        filename,
        content_type,
        size: size as i64,
        url,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Extract file from multipart form data.
///
/// Expects a single "file" field. Returns (filename, content_type, bytes).
/// Returns error if file field is missing or contains invalid data.
async fn extract_file_from_multipart(
    mut multipart: Multipart,
) -> Result<(String, String, Vec<u8>), ApiError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to read multipart field");
            ApiError::BadRequest("invalid multipart form data".into())
        })?
    {
        if field.name() == Some("file") {
            let filename = field
                .file_name()
                .ok_or_else(|| {
                    tracing::warn!("file field missing filename");
                    ApiError::BadRequest("file field must have a filename".into())
                })?
                .to_string();

            let content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();

            let bytes = field.bytes().await.map_err(|e| {
                tracing::error!(error = %e, "failed to read file bytes");
                ApiError::BadRequest("failed to read file contents".into())
            })?;

            return Ok((filename, content_type, bytes.to_vec()));
        }
    }

    tracing::warn!("multipart form missing 'file' field");
    Err(ApiError::BadRequest(
        "multipart form must contain a 'file' field".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attachment_response_serialization() {
        let response = AttachmentResponse {
            id: "123456789".to_string(),
            filename: "test.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            size: 1024,
            url: "http://minio:9000/opencorde/attachments/uuid/test.pdf".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test.pdf"));
        assert!(json.contains("application/pdf"));
        assert!(json.contains("1024"));
    }
}
