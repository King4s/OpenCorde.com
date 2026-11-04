//! GET /api/v1/friends/blocked handler.

use axum::{Json, extract::State};
use opencorde_db::repos::relationship_repo;

use super::types::{BlockedListResponse, RelationshipResponse};
use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

/// GET /api/v1/friends/blocked — List all blocked users.
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn list_blocked(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<BlockedListResponse>, ApiError> {
    let blocked = relationship_repo::list_blocked(&state.db, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to list blocked users");
            ApiError::InternalServerError("failed to list blocked users".into())
        })?;

    let count = blocked.len();

    let responses: Vec<RelationshipResponse> = blocked
        .into_iter()
        .map(|rel| RelationshipResponse {
            id: rel.id.to_string(),
            from_user: rel.from_user.to_string(),
            to_user: rel.to_user.to_string(),
            status: rel.status,
            other_username: rel.other_username,
            other_avatar_url: rel.other_avatar_url,
            created_at: rel.created_at,
        })
        .collect();

    Ok(Json(BlockedListResponse {
        blocked: responses,
        count,
    }))
}
