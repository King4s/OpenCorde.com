//! PATCH /api/v1/applications/{id} — update application (owner only).

use axum::{
    Json,
    extract::{Path, State},
};
use opencorde_core::snowflake::Snowflake;
use opencorde_db::repos::app_repo;

use super::types::{ApplicationResponse, UpdateApplicationRequest};
use crate::routes::helpers::parse_snowflake;
use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

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

pub async fn update_application(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id_str): Path<String>,
    Json(req): Json<UpdateApplicationRequest>,
) -> Result<Json<ApplicationResponse>, ApiError> {
    let app_id = parse_snowflake(&id_str)?;

    let row = app_repo::get_by_id(&state.db, app_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("application not found".to_string()))?;

    if row.owner_user_id != auth.user_id.as_i64() {
        return Err(ApiError::Forbidden);
    }

    let name = req.name.unwrap_or(row.name);
    let description = req.description.or(row.description);
    let icon_url = req.icon_url.or(row.icon_url);
    let is_public = req.is_public.unwrap_or(row.is_public);
    let redirect_uris: Vec<String> = req
        .redirect_uris
        .unwrap_or_else(|| serde_json::from_value(row.redirect_uris.clone()).unwrap_or_default());

    app_repo::update_application(
        &state.db,
        app_id,
        &name,
        description.as_deref(),
        icon_url.as_deref(),
        is_public,
        &redirect_uris,
    )
    .await
    .map_err(|e| {
        if e.to_string().contains("duplicate key") {
            ApiError::BadRequest(format!("application name '{}' already exists", name))
        } else {
            ApiError::Database(e)
        }
    })?;

    // Fetch updated row
    let updated = app_repo::get_by_id(&state.db, app_id)
        .await
        .map_err(ApiError::Database)?
        .unwrap();

    Ok(Json(ApplicationResponse::from(row_to_app(updated))))
}
