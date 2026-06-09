//! PUT /api/v1/friends/{relationship_id}/nickname handler.

use axum::{
    extract::{Path, State},
    Json,
};
use opencorde_core::snowflake::Snowflake;
use opencorde_db::repos::relationship_repo;

use super::types::{NicknameRequest, RelationshipResponse};
use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

/// PUT /api/v1/friends/{relationship_id}/nickname — Set or clear a nickname.
#[tracing::instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
pub async fn set_nickname(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(relationship_id): Path<String>,
    Json(req): Json<NicknameRequest>,
) -> Result<Json<RelationshipResponse>, ApiError> {
    let rel_id = relationship_id
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("invalid relationship_id format".into()))
        .map(Snowflake::new)?;

    // Verify the user is part of this relationship
    let rel = sqlx::query_as::<_, (i64, i64)>("SELECT from_user, to_user FROM relationships WHERE id=$1")
        .bind(rel_id.as_i64())
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch relationship");
            ApiError::InternalServerError("database error".into())
        })?
        .ok_or_else(|| {
            ApiError::NotFound("relationship not found".into())
        })?;

    let auth_user_i64 = auth.user_id.as_i64();
    if rel.0 != auth_user_i64 && rel.1 != auth_user_i64 {
        return Err(ApiError::BadRequest(
            "cannot set nickname on relationship you are not part of".into(),
        ));
    }

    let updated = relationship_repo::set_nickname(&state.db, rel_id, &req.nickname)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to set nickname");
            ApiError::InternalServerError("failed to set nickname".into())
        })?;

    Ok(Json(RelationshipResponse {
        id: updated.id.to_string(),
        from_user: updated.from_user.to_string(),
        to_user: updated.to_user.to_string(),
        status: updated.status,
        other_username: updated.other_username,
        other_avatar_url: updated.other_avatar_url,
        created_at: updated.created_at,
    }))
}

/// DELETE /api/v1/friends/{relationship_id}/nickname — Clear a nickname.
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn clear_nickname(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(relationship_id): Path<String>,
) -> Result<Json<RelationshipResponse>, ApiError> {
    let rel_id = relationship_id
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("invalid relationship_id format".into()))
        .map(Snowflake::new)?;

    let rel = sqlx::query_as::<_, (i64, i64)>("SELECT from_user, to_user FROM relationships WHERE id=$1")
        .bind(rel_id.as_i64())
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch relationship");
            ApiError::InternalServerError("database error".into())
        })?
        .ok_or_else(|| {
            ApiError::NotFound("relationship not found".into())
        })?;

    let auth_user_i64 = auth.user_id.as_i64();
    if rel.0 != auth_user_i64 && rel.1 != auth_user_i64 {
        return Err(ApiError::BadRequest(
            "cannot clear nickname on relationship you are not part of".into(),
        ));
    }

    let updated = relationship_repo::set_nickname(&state.db, rel_id, "")
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to clear nickname");
            ApiError::InternalServerError("failed to clear nickname".into())
        })?;

    Ok(Json(RelationshipResponse {
        id: updated.id.to_string(),
        from_user: updated.from_user.to_string(),
        to_user: updated.to_user.to_string(),
        status: updated.status,
        other_username: updated.other_username,
        other_avatar_url: updated.other_avatar_url,
        created_at: updated.created_at,
    }))
}
