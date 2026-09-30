//! GET /api/v1/friends/{friend_id}/mutual-friends and /mutual-servers handlers.

use axum::{
    extract::{Path, State},
    Json,
};
use opencorde_core::snowflake::Snowflake;
use opencorde_db::repos::relationship_repo;

use super::types::{
    MutualFriendsResponse, MutualServerResponse, MutualServersResponse, RelationshipResponse,
};
use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

/// GET /api/v1/friends/{friend_id}/mutual-friends — Get mutual friends.
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn get_mutual_friends(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(friend_id): Path<String>,
) -> Result<Json<MutualFriendsResponse>, ApiError> {
    let friend_id = friend_id
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("invalid friend_id format".into()))
        .map(Snowflake::new)?;

    let mutuals = relationship_repo::get_mutual_friends(&state.db, auth.user_id, friend_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to get mutual friends");
            ApiError::InternalServerError("failed to get mutual friends".into())
        })?;

    let count = mutuals.len();

    let friends: Vec<RelationshipResponse> = mutuals
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

    Ok(Json(MutualFriendsResponse { friends, count }))
}

/// GET /api/v1/friends/{friend_id}/mutual-servers — Get mutual servers.
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn get_mutual_servers(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(friend_id): Path<String>,
) -> Result<Json<MutualServersResponse>, ApiError> {
    let friend_id = friend_id
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("invalid friend_id format".into()))
        .map(Snowflake::new)?;

    let servers = relationship_repo::get_mutual_servers(&state.db, auth.user_id, friend_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to get mutual servers");
            ApiError::InternalServerError("failed to get mutual servers".into())
        })?;

    let count = servers.len();

    let responses: Vec<MutualServerResponse> = servers
        .into_iter()
        .map(|s| MutualServerResponse {
            id: s.server_id.to_string(),
            name: s.server_name,
            icon_url: s.icon_url,
        })
        .collect();

    Ok(Json(MutualServersResponse {
        servers: responses,
        count,
    }))
}
