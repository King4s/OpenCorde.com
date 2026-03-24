//! DELETE /api/v1/friends/{relationship_id} handler.

use axum::{extract::{State, Path}, http::StatusCode};
use opencorde_core::snowflake::Snowflake;
use opencorde_db::repos::relationship_repo;

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};

/// DELETE /api/v1/friends/{relationship_id} — Remove a friend or decline a request.
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn remove_relationship(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(relationship_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(relationship_id = %relationship_id, "removing relationship");

    let rel_id = relationship_id
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("invalid relationship_id format".into()))
        .map(Snowflake::new)?;

    let rel = sqlx::query_as::<_, (i64, i64)>(
        "SELECT from_user, to_user FROM relationships WHERE id=$1"
    )
    .bind(rel_id.as_i64())
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to fetch relationship");
        ApiError::InternalServerError("database error".into())
    })?
    .ok_or_else(|| {
        tracing::warn!(rel_id = %rel_id.as_i64(), "relationship not found");
        ApiError::NotFound("relationship not found".into())
    })?;

    let auth_user_i64 = auth.user_id.as_i64();
    if rel.0 != auth_user_i64 && rel.1 != auth_user_i64 {
        tracing::warn!(rel_id = %rel_id.as_i64(), "unauthorized remove attempt");
        return Err(ApiError::BadRequest(
            "cannot remove relationship you are not part of".into(),
        ));
    }

    relationship_repo::delete_relationship(&state.db, rel_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to delete relationship");
            ApiError::InternalServerError("failed to remove relationship".into())
        })?;

    Ok(StatusCode::NO_CONTENT)
}
