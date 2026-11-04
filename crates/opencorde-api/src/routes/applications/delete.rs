//! DELETE /api/v1/applications/{id} — delete application (owner only).

use axum::{extract::{Path, State}, http::StatusCode};
use opencorde_db::repos::app_repo;

use crate::routes::helpers::parse_snowflake;
use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

pub async fn delete_application(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id_str): Path<String>,
) -> Result<StatusCode, ApiError> {
    let app_id = parse_snowflake(&id_str)?;

    let row = app_repo::get_by_id(&state.db, app_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("application not found".to_string()))?;

    if row.owner_user_id != auth.user_id.as_i64() {
        return Err(ApiError::Forbidden);
    }

    app_repo::delete_application(&state.db, app_id)
        .await
        .map_err(ApiError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}
