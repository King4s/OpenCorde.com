//! # Route: Server Emojis
//! Custom emoji upload, listing, and deletion.
//!
//! ## Endpoints
//! - POST /api/v1/servers/{id}/emojis — Upload emoji (multipart, owner only)
//! - GET /api/v1/servers/{id}/emojis — List server emojis (auth)
//! - DELETE /api/v1/servers/{id}/emojis/{emoji_id} — Delete emoji (owner only)
//!
//! ## Depends On
//! - axum, serde, opencorde_db, crate::AppState, aws_sdk_s3

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    routing::{delete, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};
use crate::routes::helpers::parse_snowflake;
use crate::emoji_helpers::{
    extract_emoji_from_multipart, is_valid_emoji_name, get_emoji_extension,
    MAX_EMOJI_SIZE, VALID_EMOJI_CONTENT_TYPES,
};

/// Response body for emoji data.
#[derive(Debug, Serialize, Deserialize)]
pub struct EmojiResponse {
    pub id: String,
    pub server_id: String,
    pub name: String,
    pub image_url: String,
    pub uploaded_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Build the emojis router with all endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/servers/{id}/emojis",
            post(upload_emoji).get(list_emojis),
        )
        .route(
            "/api/v1/servers/{id}/emojis/{emoji_id}",
            delete(delete_emoji),
        )
}

/// POST /api/v1/servers/{id}/emojis — Upload an emoji.
#[instrument(skip(state, auth, multipart), fields(user_id = %auth.user_id))]
async fn upload_emoji(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
    multipart: Multipart,
) -> Result<(StatusCode, Json<EmojiResponse>), ApiError> {
    let server_id_i64 = parse_snowflake(&server_id)?.as_i64();
    tracing::info!(server_id = server_id_i64, "uploading emoji");

    // Verify ownership
    check_server_owner(&state, auth.user_id.as_i64(), server_id_i64).await?;

    // Extract multipart fields
    let (name, content_type, bytes) = extract_emoji_from_multipart(multipart).await?;

    tracing::debug!(
        emoji_name = %name,
        content_type = %content_type,
        size = bytes.len(),
        "emoji extracted from multipart"
    );

    // Validate inputs
    if !is_valid_emoji_name(&name) {
        tracing::warn!(name = %name, "invalid emoji name");
        return Err(ApiError::BadRequest(
            "emoji name must be 2-32 characters, lowercase letters, numbers, and underscores only"
                .into(),
        ));
    }

    if !VALID_EMOJI_CONTENT_TYPES.contains(&content_type.as_str()) {
        tracing::warn!(content_type = %content_type, "invalid emoji content type");
        return Err(ApiError::BadRequest("emoji must be PNG, GIF, or WebP".into()));
    }

    let size = bytes.len() as u64;
    if size > MAX_EMOJI_SIZE {
        tracing::warn!(size = size, max = MAX_EMOJI_SIZE, "emoji exceeds maximum size");
        return Err(ApiError::BadRequest(
            "emoji size exceeds maximum of 256KB".into(),
        ));
    }

    // Generate IDs and S3 key
    let mut generator = opencorde_core::snowflake::SnowflakeGenerator::new(1, 1);
    let emoji_id_i64 = generator.next_id().as_i64();
    let ext = get_emoji_extension(&content_type);
    let object_key = format!("emojis/{}/{}.{}", server_id_i64, emoji_id_i64, ext);

    // Upload to S3
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
            ApiError::Internal(anyhow::anyhow!("failed to upload emoji to S3: {}", e))
        })?;

    let image_url = format!(
        "{}/{}/{}",
        state.config.minio_endpoint, state.config.minio_bucket, object_key
    );

    // Insert into database
    opencorde_db::repos::emoji_repo::create_emoji(
        &state.db,
        emoji_id_i64,
        server_id_i64,
        &name,
        &image_url,
        auth.user_id.as_i64(),
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "emoji insert failed");
        if e.to_string().contains("unique") {
            ApiError::Conflict(format!("emoji '{}' already exists", name))
        } else {
            ApiError::Database(e)
        }
    })?;

    tracing::info!(emoji_id = emoji_id_i64, emoji_name = %name, "emoji created");

    let response = EmojiResponse {
        id: emoji_id_i64.to_string(),
        server_id: server_id_i64.to_string(),
        name,
        image_url,
        uploaded_by: auth.user_id.as_i64().to_string(),
        created_at: chrono::Utc::now(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/v1/servers/{id}/emojis — List server emojis.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn list_emojis(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
) -> Result<Json<Vec<EmojiResponse>>, ApiError> {
    let server_id_i64 = parse_snowflake(&server_id)?.as_i64();
    tracing::info!(server_id = server_id_i64, "listing emojis");

    // Verify server exists
    verify_server_exists(&state, server_id_i64).await?;

    let rows = opencorde_db::repos::emoji_repo::list_emojis(&state.db, server_id_i64)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "emoji list fetch failed");
            ApiError::Database(e)
        })?;

    let response: Vec<EmojiResponse> = rows
        .into_iter()
        .map(|row| EmojiResponse {
            id: row.id.to_string(),
            server_id: row.server_id.to_string(),
            name: row.name,
            image_url: row.image_url,
            uploaded_by: row.uploaded_by.to_string(),
            created_at: row.created_at,
        })
        .collect();

    tracing::debug!(count = response.len(), "emojis listed");
    Ok(Json(response))
}

/// DELETE /api/v1/servers/{id}/emojis/{emoji_id} — Delete an emoji.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn delete_emoji(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, emoji_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let server_id_i64 = parse_snowflake(&server_id)?.as_i64();
    let emoji_id_i64 = parse_snowflake(&emoji_id)?.as_i64();

    tracing::info!(
        server_id = server_id_i64,
        emoji_id = emoji_id_i64,
        "deleting emoji"
    );

    // Verify ownership
    check_server_owner(&state, auth.user_id.as_i64(), server_id_i64).await?;

    let deleted = opencorde_db::repos::emoji_repo::delete_emoji(
        &state.db,
        emoji_id_i64,
        server_id_i64,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "emoji deletion failed");
        ApiError::Database(e)
    })?;

    if !deleted {
        tracing::warn!(emoji_id = emoji_id_i64, "emoji not found");
        return Err(ApiError::NotFound("emoji not found".into()));
    }

    tracing::info!(emoji_id = emoji_id_i64, "emoji deleted");
    Ok(StatusCode::NO_CONTENT)
}

/// Check if user is server owner.
async fn check_server_owner(
    state: &AppState,
    user_id: i64,
    server_id: i64,
) -> Result<(), ApiError> {
    let owner_id: (i64,) = sqlx::query_as("SELECT owner_id FROM servers WHERE id = $1")
        .bind(server_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch server owner");
            ApiError::Database(e)
        })?
        .ok_or_else(|| {
            tracing::warn!(server_id = server_id, "server not found");
            ApiError::NotFound("server not found".into())
        })?;

    if user_id != owner_id.0 {
        tracing::warn!(user_id = user_id, owner_id = owner_id.0, "user is not server owner");
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

/// Verify server exists.
async fn verify_server_exists(state: &AppState, server_id: i64) -> Result<(), ApiError> {
    let _: (i64,) = sqlx::query_as("SELECT id FROM servers WHERE id = $1")
        .bind(server_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch server");
            ApiError::Database(e)
        })?
        .ok_or_else(|| {
            tracing::warn!(server_id = server_id, "server not found");
            ApiError::NotFound("server not found".into())
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emoji_response_serialization() {
        let response = EmojiResponse {
            id: "123456789".to_string(),
            server_id: "987654321".to_string(),
            name: "happy".to_string(),
            image_url: "http://minio:9000/opencorde/emojis/987654321/123456789.png".to_string(),
            uploaded_by: "111111111".to_string(),
            created_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("happy"));
        assert!(json.contains("123456789"));
    }
}
