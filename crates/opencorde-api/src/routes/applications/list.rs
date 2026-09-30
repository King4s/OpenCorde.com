//! GET /api/v1/applications/@me and GET /api/v1/applications/public

use axum::{Json, extract::State};
use opencorde_core::snowflake::Snowflake;
use opencorde_db::repos::app_repo;

use super::types::{ApplicationResponse, ListMineResponse, ListPublicResponse};
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

#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn list_my_applications(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ListMineResponse>, ApiError> {
    let rows = app_repo::list_by_owner(&state.db, Snowflake::new(auth.user_id.as_i64()))
        .await
        .map_err(ApiError::Database)?;

    let apps: Vec<ApplicationResponse> = rows
        .into_iter()
        .map(|r| ApplicationResponse::from(row_to_app(r)))
        .collect();

    Ok(Json(ListMineResponse { applications: apps }))
}

pub async fn list_public_applications(
    State(state): State<AppState>,
) -> Result<Json<ListPublicResponse>, ApiError> {
    let rows: Vec<app_repo::ApplicationRow> = sqlx::query_as::<_, app_repo::ApplicationRow>(
        "SELECT * FROM applications WHERE is_public = true ORDER BY created_at DESC LIMIT 100",
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let apps: Vec<ApplicationResponse> = rows
        .into_iter()
        .map(|r| ApplicationResponse::from(row_to_app(r)))
        .collect();

    Ok(Json(ListPublicResponse { applications: apps }))
}
