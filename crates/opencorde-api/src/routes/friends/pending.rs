//! GET /api/v1/friends/pending handler.

use axum::{Json, extract::State};
use opencorde_db::repos::relationship_repo;

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};
use super::types::{PendingResponse, RelationshipResponse};

/// GET /api/v1/friends/pending — List pending incoming and outgoing requests.
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn list_pending(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<PendingResponse>, ApiError> {
    tracing::info!("listing pending requests");

    let (incoming, outgoing) = tokio::join!(
        relationship_repo::list_pending_incoming(&state.db, auth.user_id),
        relationship_repo::list_pending_outgoing(&state.db, auth.user_id)
    );

    let incoming = incoming.map_err(|e| {
        tracing::error!(error = %e, "failed to list incoming requests");
        ApiError::InternalServerError("failed to list requests".into())
    })?;

    let outgoing = outgoing.map_err(|e| {
        tracing::error!(error = %e, "failed to list outgoing requests");
        ApiError::InternalServerError("failed to list requests".into())
    })?;

    let incoming_responses = incoming
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

    let outgoing_responses = outgoing
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

    Ok(Json(PendingResponse {
        incoming: incoming_responses,
        outgoing: outgoing_responses,
    }))
}
