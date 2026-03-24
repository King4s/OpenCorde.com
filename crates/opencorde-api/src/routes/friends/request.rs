//! POST /api/v1/friends/request handler.

use axum::{Json, extract::State, http::StatusCode};
use opencorde_core::snowflake::SnowflakeGenerator;
use opencorde_db::repos::{relationship_repo, user_repo};

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};
use super::types::{UserIdRequest, RelationshipResponse};

/// POST /api/v1/friends/request — Send a friend request.
#[tracing::instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
pub async fn send_request(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UserIdRequest>,
) -> Result<(StatusCode, Json<RelationshipResponse>), ApiError> {
    tracing::info!(target_id = %req.user_id, "sending friend request");

    let target_id = req
        .user_id
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("invalid user_id format".into()))
        .map(opencorde_core::snowflake::Snowflake::new)?;

    if target_id == auth.user_id {
        return Err(ApiError::BadRequest("cannot add yourself".into()));
    }

    user_repo::get_by_id(&state.db, target_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to check target user");
            ApiError::InternalServerError("database error".into())
        })?
        .ok_or_else(|| {
            tracing::warn!(target_id = %target_id.as_i64(), "target user not found");
            ApiError::NotFound("user not found".into())
        })?;

    let mut generator = SnowflakeGenerator::new(1, 1);
    let rel_id = generator.next_id();

    let rel = relationship_repo::send_request(&state.db, rel_id, auth.user_id, target_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to send friend request");
            if e.to_string().contains("duplicate") {
                ApiError::BadRequest("relationship already exists".into())
            } else {
                ApiError::InternalServerError("failed to send request".into())
            }
        })?;

    let response = RelationshipResponse {
        id: rel.id.to_string(),
        from_user: rel.from_user.to_string(),
        to_user: rel.to_user.to_string(),
        status: rel.status,
        other_username: rel.other_username,
        other_avatar_url: rel.other_avatar_url,
        created_at: rel.created_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}
