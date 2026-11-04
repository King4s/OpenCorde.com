//! POST /api/v1/applications — create a new application.

use axum::{extract::State, http::StatusCode, Json};
use opencorde_core::snowflake::SnowflakeGenerator;
use opencorde_db::repos::app_repo;
use opencorde_core::snowflake::Snowflake;
use opencorde_core::models::application::Application;

use super::types::{ApplicationResponse, CreateApplicationRequest};
use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

pub async fn create_application(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateApplicationRequest>,
) -> Result<(StatusCode, Json<ApplicationResponse>), ApiError> {
    let mut generator = SnowflakeGenerator::new(5, 0);
    let app_id = generator.next_id();

    let row = app_repo::create_application(
        &state.db, app_id, req.name.trim(),
        req.description.as_deref(), req.icon_url.as_deref(),
        Snowflake::new(auth.user_id.as_i64()), 0, req.is_public,
        &req.redirect_uris,
    ).await.map_err(|e| {
        if e.to_string().contains("duplicate key") {
            ApiError::BadRequest(format!("application '{}' already exists", req.name))
        } else { ApiError::Database(e) }
    })?;

    let redirect_uris: Vec<String> = serde_json::from_value(row.redirect_uris).unwrap_or_default();
    let app = Application {
        id: Snowflake::new(row.id), name: row.name,
        description: row.description, icon_url: row.icon_url,
        owner_user_id: Snowflake::new(row.owner_user_id),
        flags: row.flags, is_public: row.is_public,
        redirect_uris, created_at: row.created_at, updated_at: row.updated_at,
    };

    Ok((StatusCode::CREATED, Json(ApplicationResponse::from(app))))
}
