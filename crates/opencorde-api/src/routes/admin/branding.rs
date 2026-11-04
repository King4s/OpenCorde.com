//! # Admin Branding Handlers
//! Endpoints for managing instance branding — name, tagline, color,
//! logo, favicon, and custom CSS.
//!
//! ## Endpoints
//! - GET  /api/v1/admin/branding        — Return current branding settings
//! - PUT  /api/v1/admin/branding        — Update branding settings
//! - POST /api/v1/admin/branding/logo   — Upload instance logo (multipart)
//! - POST /api/v1/admin/branding/favicon — Upload instance favicon (multipart)
//! - POST /api/v1/admin/branding/reset  — Reset to defaults
//!
//! ## Depends On
//! - crate::middleware::auth::AuthUser
//! - crate::AppState
//! - aws_sdk_s3 for file storage

use axum::{
    Json, Router,
    extract::{Multipart, State},
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};
use crate::routes::upload_validation;

use super::handlers::is_admin;

/// Valid image content types for logo and favicon.
const VALID_BRANDING_IMAGE_TYPES: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/webp",
    "image/svg+xml",
    "image/x-icon",
    "image/vnd.microsoft.icon",
];

/// Maximum size for branding images: 2 MB.
const MAX_BRANDING_IMAGE_SIZE: u64 = 2 * 1024 * 1024;

// ------------------------------------------------------------------
// Types
// ------------------------------------------------------------------

/// Branding settings as stored in the database.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceBranding {
    pub instance_name: Option<String>,
    pub tagline: Option<String>,
    pub primary_color: Option<String>,
    pub logo_url: Option<String>,
    pub favicon_url: Option<String>,
    pub custom_css: Option<String>,
    pub base_url: Option<String>,
}

/// Request body for PUT /api/v1/admin/branding.
#[derive(Debug, Deserialize)]
pub struct UpdateBrandingRequest {
    pub instance_name: Option<String>,
    pub tagline: Option<String>,
    pub primary_color: Option<String>,
    pub logo_url: Option<String>,
    pub favicon_url: Option<String>,
    pub custom_css: Option<String>,
    pub base_url: Option<String>,
}

/// Response after a branding image upload.
#[derive(Debug, Serialize)]
pub struct BrandingImageUploadResponse {
    pub url: String,
    pub filename: String,
    pub content_type: String,
    pub size: u64,
}

// ------------------------------------------------------------------
// DB helpers
// ------------------------------------------------------------------

async fn get_branding_db(db: &sqlx::PgPool) -> Result<InstanceBranding, ApiError> {
    let row = sqlx::query(
        "SELECT instance_name, tagline, primary_color, logo_url, favicon_url, custom_css, base_url
         FROM instance_setup WHERE id = TRUE",
    )
    .fetch_optional(db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    match row {
        Some(r) => Ok(InstanceBranding {
            instance_name: r.get("instance_name"),
            tagline: r.get("tagline"),
            primary_color: r.get("primary_color"),
            logo_url: r.get("logo_url"),
            favicon_url: r.get("favicon_url"),
            custom_css: r.get("custom_css"),
            base_url: r.get("base_url"),
        }),
        None => {
            // Instance setup row doesn't exist yet — return empty branding.
            Ok(InstanceBranding {
                instance_name: None,
                tagline: None,
                primary_color: None,
                logo_url: None,
                favicon_url: None,
                custom_css: None,
                base_url: None,
            })
        }
    }
}

async fn update_branding_db(
    db: &sqlx::PgPool,
    req: &UpdateBrandingRequest,
) -> Result<InstanceBranding, ApiError> {
    // Build instance_setup row if it doesn't exist
    sqlx::query(
        "INSERT INTO instance_setup (id)
         VALUES (TRUE)
         ON CONFLICT (id) DO NOTHING",
    )
    .execute(db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    // Update only provided fields
    let row = sqlx::query(
        "UPDATE instance_setup SET
           instance_name = COALESCE($1, instance_name),
           tagline       = COALESCE($2, tagline),
           primary_color = COALESCE($3, primary_color),
           logo_url      = COALESCE($4, logo_url),
           favicon_url   = COALESCE($5, favicon_url),
           custom_css    = COALESCE($6, custom_css),
           base_url      = COALESCE($7, base_url)
         WHERE id = TRUE
         RETURNING instance_name, tagline, primary_color, logo_url, favicon_url, custom_css, base_url",
    )
    .bind(&req.instance_name)
    .bind(&req.tagline)
    .bind(&req.primary_color)
    .bind(&req.logo_url)
    .bind(&req.favicon_url)
    .bind(&req.custom_css)
    .bind(&req.base_url)
    .fetch_one(db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(InstanceBranding {
        instance_name: row.get("instance_name"),
        tagline: row.get("tagline"),
        primary_color: row.get("primary_color"),
        logo_url: row.get("logo_url"),
        favicon_url: row.get("favicon_url"),
        custom_css: row.get("custom_css"),
        base_url: row.get("base_url"),
    })
}

/// Extract a single file from a multipart form.
async fn extract_branding_image(
    multipart: Multipart,
) -> Result<(String, String, Vec<u8>), ApiError> {
    let mut field_name: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut data: Vec<u8> = Vec::new();

    let mut mp = multipart;
    while let Ok(Some(mut field)) = mp.next_field().await {
        let name = field.name().unwrap_or("file").to_string();
        let ct = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();
        field_name = Some(name);

        // Only take the content type from the first field with a recognized image ct
        if content_type.is_none()
            || (VALID_BRANDING_IMAGE_TYPES.contains(&ct.as_str())
                && !VALID_BRANDING_IMAGE_TYPES.contains(
                    &content_type
                        .as_deref()
                        .unwrap_or("application/octet-stream"),
                ))
        {
            content_type = Some(ct);
        }

        while let Ok(Some(chunk)) = field.chunk().await {
            data.extend_from_slice(&chunk);
            if data.len() as u64 > MAX_BRANDING_IMAGE_SIZE + 1024 {
                return Err(ApiError::BadRequest(format!(
                    "file exceeds maximum size of {} bytes",
                    MAX_BRANDING_IMAGE_SIZE
                )));
            }
        }
    }

    let ct = content_type.unwrap_or_else(|| "application/octet-stream".to_string());
    let _name = field_name.unwrap_or_else(|| "file".to_string());

    Ok((_name, ct, data))
}

/// Upload image bytes to S3/MinIO and return its public URL.
async fn upload_branding_image_to_s3(
    state: &AppState,
    folder: &str,
    content_type: &str,
    data: Vec<u8>,
) -> Result<String, ApiError> {
    let ext = match content_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        "image/x-icon" | "image/vnd.microsoft.icon" => "ico",
        _ => "png",
    };

    let object_key = format!("branding/{}/{}.{}", folder, Uuid::new_v4(), ext);
    let size = data.len();

    state
        .s3
        .put_object()
        .bucket(&state.config.minio_bucket)
        .key(&object_key)
        .body(data.into())
        .content_type(content_type)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "S3 upload failed for branding image");
            ApiError::Internal(anyhow::anyhow!("failed to upload branding image: {}", e))
        })?;

    let url = format!(
        "{}/{}/{}",
        state.config.minio_endpoint,
        state.config.minio_bucket,
        object_key
    );

    tracing::info!(url = %url, size = size, content_type = %content_type, "branding image uploaded");

    Ok(url)
}

// ------------------------------------------------------------------
// GET /api/v1/admin/branding
// ------------------------------------------------------------------

/// GET /api/v1/admin/branding — Return current branding settings.
///
/// Requires admin role. Returns all fields, empty string for unset values.
#[tracing::instrument(skip(state, auth))]
pub async fn get_branding(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<InstanceBranding>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "admin: fetching branding");

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted branding read");
        return Err(ApiError::Forbidden);
    }

    let branding = get_branding_db(&state.db).await?;

    tracing::info!(instance_name = ?branding.instance_name, "admin: branding fetched");
    Ok(Json(branding))
}

// ------------------------------------------------------------------
// PUT /api/v1/admin/branding
// ------------------------------------------------------------------

/// PUT /api/v1/admin/branding — Update branding settings.
///
/// Requires admin role. Only provided fields are updated; omitted fields
/// keep their current values. The instance_setup row is created if it
/// doesn't exist yet.
#[tracing::instrument(skip(state, auth, body))]
pub async fn update_branding(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<UpdateBrandingRequest>,
) -> Result<Json<InstanceBranding>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "admin: updating branding");

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted branding update");
        return Err(ApiError::Forbidden);
    }

    // Validation
    if let Some(ref name) = body.instance_name {
        if name.len() > 128 {
            return Err(ApiError::BadRequest(
                "instance_name must not exceed 128 characters".into(),
            ));
        }
    }
    if let Some(ref tagline) = body.tagline {
        if tagline.len() > 256 {
            return Err(ApiError::BadRequest(
                "tagline must not exceed 256 characters".into(),
            ));
        }
    }
    if let Some(ref color) = body.primary_color {
        if color.len() > 32 || !color.starts_with('#') {
            return Err(ApiError::BadRequest(
                "primary_color must be a hex color like #5865F2".into(),
            ));
        }
    }
    if let Some(ref css) = body.custom_css {
        if css.len() > 65536 {
            return Err(ApiError::BadRequest(
                "custom_css must not exceed 64KB".into(),
            ));
        }
    }
    if let Some(ref url) = body.base_url {
        if url.len() > 256 {
            return Err(ApiError::BadRequest(
                "base_url must not exceed 256 characters".into(),
            ));
        }
    }

    let result = update_branding_db(&state.db, &body).await?;

    tracing::info!(instance_name = ?result.instance_name, "admin: branding updated");
    Ok(Json(result))
}

// ------------------------------------------------------------------
// POST /api/v1/admin/branding/logo
// ------------------------------------------------------------------

/// POST /api/v1/admin/branding/logo — Upload instance logo.
///
/// Requires admin role. Accepts multipart/form-data with a single image file.
/// Valid types: PNG, JPEG, WebP, SVG. Max size: 2 MB.
/// Updates logo_url in instance_setup automatically.
#[tracing::instrument(skip(state, auth, multipart))]
pub async fn upload_logo(
    State(state): State<AppState>,
    auth: AuthUser,
    multipart: Multipart,
) -> Result<Json<BrandingImageUploadResponse>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "admin: uploading logo");

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted logo upload");
        return Err(ApiError::Forbidden);
    }

    let (filename, content_type, data) = extract_branding_image(multipart).await?;

    // Validate content type
    if !VALID_BRANDING_IMAGE_TYPES.contains(&content_type.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "unsupported image type: {}. Must be PNG, JPEG, WebP, SVG, or ICO",
            content_type
        )));
    }

    // Validate size
    let size = data.len() as u64;
    if size > MAX_BRANDING_IMAGE_SIZE {
        return Err(ApiError::BadRequest(format!(
            "file exceeds maximum size of {} bytes",
            MAX_BRANDING_IMAGE_SIZE
        )));
    }

    // Validate magic bytes
    upload_validation::verify_magic_bytes(&content_type, &data)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let url = upload_branding_image_to_s3(&state, "logo", &content_type, data).await?;

    // Update the logo_url in instance_setup
    update_branding_db(
        &state.db,
        &UpdateBrandingRequest {
            instance_name: None,
            tagline: None,
            primary_color: None,
            logo_url: Some(url.clone()),
            favicon_url: None,
            custom_css: None,
            base_url: None,
        },
    )
    .await?;

    Ok(Json(BrandingImageUploadResponse {
        url,
        filename,
        content_type,
        size,
    }))
}

// ------------------------------------------------------------------
// POST /api/v1/admin/branding/favicon
// ------------------------------------------------------------------

/// POST /api/v1/admin/branding/favicon — Upload instance favicon.
///
/// Requires admin role. Accepts multipart/form-data with a single image file.
/// Valid types: PNG, JPEG, WebP, SVG, ICO. Max size: 2 MB.
/// Updates favicon_url in instance_setup automatically.
#[tracing::instrument(skip(state, auth, multipart))]
pub async fn upload_favicon(
    State(state): State<AppState>,
    auth: AuthUser,
    multipart: Multipart,
) -> Result<Json<BrandingImageUploadResponse>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "admin: uploading favicon");

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted favicon upload");
        return Err(ApiError::Forbidden);
    }

    let (filename, content_type, data) = extract_branding_image(multipart).await?;

    if !VALID_BRANDING_IMAGE_TYPES.contains(&content_type.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "unsupported image type: {}. Must be PNG, JPEG, WebP, SVG, or ICO",
            content_type
        )));
    }

    let size = data.len() as u64;
    if size > MAX_BRANDING_IMAGE_SIZE {
        return Err(ApiError::BadRequest(format!(
            "file exceeds maximum size of {} bytes",
            MAX_BRANDING_IMAGE_SIZE
        )));
    }

    upload_validation::verify_magic_bytes(&content_type, &data)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let url = upload_branding_image_to_s3(&state, "favicon", &content_type, data).await?;

    // Update the favicon_url in instance_setup
    update_branding_db(
        &state.db,
        &UpdateBrandingRequest {
            instance_name: None,
            tagline: None,
            primary_color: None,
            logo_url: None,
            favicon_url: Some(url.clone()),
            custom_css: None,
            base_url: None,
        },
    )
    .await?;

    Ok(Json(BrandingImageUploadResponse {
        url,
        filename,
        content_type,
        size,
    }))
}

// ------------------------------------------------------------------
// POST /api/v1/admin/branding/reset
// ------------------------------------------------------------------

/// POST /api/v1/admin/branding/reset — Reset branding to defaults.
///
/// Requires admin role. Clears all branding fields except instance_name and base_url.
#[tracing::instrument(skip(state, auth))]
pub async fn reset_branding(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<InstanceBranding>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "admin: resetting branding");

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted branding reset");
        return Err(ApiError::Forbidden);
    }

    // Clear branding fields but keep instance_name and base_url
    let row = sqlx::query(
        "UPDATE instance_setup SET
           tagline = NULL,
           primary_color = NULL,
           logo_url = NULL,
           favicon_url = NULL,
           custom_css = NULL
         WHERE id = TRUE
         RETURNING instance_name, tagline, primary_color, logo_url, favicon_url, custom_css, base_url",
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let result = match row {
        Some(r) => InstanceBranding {
            instance_name: r.get("instance_name"),
            tagline: r.get("tagline"),
            primary_color: r.get("primary_color"),
            logo_url: r.get("logo_url"),
            favicon_url: r.get("favicon_url"),
            custom_css: r.get("custom_css"),
            base_url: r.get("base_url"),
        },
        None => InstanceBranding {
            instance_name: None,
            tagline: None,
            primary_color: None,
            logo_url: None,
            favicon_url: None,
            custom_css: None,
            base_url: None,
        },
    };

    tracing::info!("admin: branding reset to defaults");
    Ok(Json(result))
}

// ------------------------------------------------------------------
// Router
// ------------------------------------------------------------------

/// Build the branding admin router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/admin/branding", get(get_branding))
        .route("/api/v1/admin/branding", put(update_branding))
        .route("/api/v1/admin/branding/logo", post(upload_logo))
        .route("/api/v1/admin/branding/favicon", post(upload_favicon))
        .route("/api/v1/admin/branding/reset", post(reset_branding))
}
