//! POST /api/v1/users/@me/avatar handler for avatar uploads.

use axum::{Json, extract::{State, Multipart}};
use uuid::Uuid;
use opencorde_db::repos::user_repo;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};
use super::get::UserProfile;

/// POST /api/v1/users/@me/avatar — Upload user avatar.
///
/// Accepts multipart/form-data with a "file" field (image only).
/// Stores in MinIO and updates avatar_url in the database.
#[tracing::instrument(skip(state, auth, multipart))]
pub async fn upload_avatar(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<UserProfile>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "uploading avatar");

    // Extract file from multipart
    let (filename, content_type, bytes) = loop {
        let field = multipart.next_field().await.map_err(|e| {
            ApiError::BadRequest(format!("invalid multipart: {}", e))
        })?;
        let Some(field) = field else {
            return Err(ApiError::BadRequest("no file field found".into()));
        };
        if field.name() == Some("file") {
            let filename = field.file_name().unwrap_or("avatar").to_string();
            let content_type = field.content_type().unwrap_or("image/png").to_string();
            if !content_type.starts_with("image/") {
                return Err(ApiError::BadRequest("avatar must be an image".into()));
            }
            let bytes = field.bytes().await.map_err(|e| {
                ApiError::BadRequest(format!("failed to read file: {}", e))
            })?;
            break (filename, content_type, bytes);
        }
    };

    // Limit 5MB for avatars
    if bytes.len() > 5 * 1024 * 1024 {
        return Err(ApiError::BadRequest("avatar must be under 5 MB".into()));
    }

    // Upload to MinIO
    let object_key = format!("avatars/{}/{}", Uuid::new_v4(), filename);
    state.s3
        .put_object()
        .bucket(&state.config.minio_bucket)
        .key(&object_key)
        .body(bytes.into())
        .content_type(&content_type)
        .send()
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("S3 upload failed: {}", e)))?;

    let avatar_url = format!("{}/{}/{}", state.config.files_public_url, state.config.minio_bucket, object_key);

    // Update user record
    sqlx::query("UPDATE users SET avatar_url = $1 WHERE id = $2")
        .bind(&avatar_url)
        .bind(auth.user_id.as_i64())
        .execute(&state.db)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(user_id = %auth.user_id, avatar_url = %avatar_url, "avatar updated");

    // Fetch and return updated profile
    let user_row = user_repo::get_by_id(&state.db, auth.user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("user not found".into()))?;

    Ok(Json(UserProfile {
        id: user_row.id.to_string(),
        username: user_row.username,
        public_key: user_row.public_key,
        email: user_row.email,
        avatar_url: user_row.avatar_url,
        status: user_row.status,
        bio: user_row.bio,
        status_message: user_row.status_message,
        totp_enabled: user_row.totp_enabled,
    }))
}
