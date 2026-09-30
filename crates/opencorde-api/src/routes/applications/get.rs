//! GET /api/v1/applications/{id} — get application by ID.

use axum::{
    Json,
    extract::{Path, State},
};
use opencorde_core::snowflake::Snowflake;
use opencorde_db::repos::app_repo;

use super::types::ApplicationResponse;
use crate::routes::helpers::parse_snowflake;
use crate::{AppState, error::ApiError};

fn row_to_app(row: app_repo::ApplicationRow) -> opencorde_core::models::application::Application {
    let redirect_uris: Vec<String> = serde_json::from_value(row.redirect_uris).unwrap_or_default();
    opencorde_core::models::application::Application {
        id: Snowflake::new(row.id),
        name: row.name,
        description: row.description,
        icon_url: row.icon_url,
        owner_user_id: Snowflake::new(row.owner_user_id),
        flags: row.flags,
        is_public: row.is_public,
        redirect_uris,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

pub async fn get_application(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
) -> Result<Json<ApplicationResponse>, ApiError> {
    let app_id = parse_snowflake(&id_str)?;
    let row = app_repo::get_by_id(&state.db, app_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("application not found".to_string()))?;

    Ok(Json(ApplicationResponse::from(row_to_app(row))))
}
