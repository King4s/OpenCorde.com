//! GET /api/v1/friends handler.

use axum::{Json, extract::State};
use opencorde_db::repos::relationship_repo;

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};
use super::types::RelationshipResponse;

/// GET /api/v1/friends — List all friends of the authenticated user.
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn list_friends(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<RelationshipResponse>>, ApiError> {
    tracing::info!("listing friends");

    let friends = relationship_repo::list_friends(&state.db, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to list friends");
            ApiError::InternalServerError("failed to list friends".into())
        })?;

    let responses = friends
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

    Ok(Json(responses))
}
