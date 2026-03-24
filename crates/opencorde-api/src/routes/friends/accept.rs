//! PUT /api/v1/friends/{relationship_id}/accept handler.

use axum::{extract::{State, Path}, http::StatusCode};
use opencorde_core::snowflake::Snowflake;
use opencorde_db::repos::relationship_repo;

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};

/// PUT /api/v1/friends/{relationship_id}/accept — Accept a friend request.
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn accept_request(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(relationship_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(relationship_id = %relationship_id, "accepting friend request");

    let rel_id = relationship_id
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("invalid relationship_id format".into()))
        .map(Snowflake::new)?;

    let rel = sqlx::query_as::<_, (i64,)>(
        "SELECT to_user FROM relationships WHERE id=$1"
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

    if rel.0 != auth.user_id.as_i64() {
        tracing::warn!(rel_id = %rel_id.as_i64(), "unauthorized accept attempt");
        return Err(ApiError::BadRequest("cannot accept request not addressed to you".into()));
    }

    relationship_repo::accept_request(&state.db, rel_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to accept request");
            ApiError::InternalServerError("failed to accept request".into())
        })?;

    Ok(StatusCode::NO_CONTENT)
}
