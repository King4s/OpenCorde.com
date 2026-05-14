//! GET /api/v1/users/search handler.

use axum::{
    Json,
    extract::{Query, State},
};
use std::collections::HashMap;

use super::types::UserSearchResult;
use crate::{AppState, error::ApiError};

/// GET /api/v1/users/search?q={query} — Search users by username.
#[tracing::instrument(skip(state), fields(query_len = 0))]
pub async fn search_users(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<UserSearchResult>>, ApiError> {
    let q = params
        .get("q")
        .ok_or_else(|| ApiError::BadRequest("q parameter required".into()))?;

    if q.len() < 2 {
        return Err(ApiError::BadRequest(
            "query must be at least 2 characters".into(),
        ));
    }

    tracing::info!(query = %q, "searching users");

    let rows = sqlx::query_as::<_, (i64, String, Option<String>)>(
        "SELECT id, username, avatar_url FROM users WHERE username ILIKE $1 LIMIT 20",
    )
    .bind(format!("%{}%", q))
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to search users");
        ApiError::InternalServerError("failed to search users".into())
    })?;

    let results = rows
        .into_iter()
        .map(|(id, username, avatar_url)| UserSearchResult {
            id: id.to_string(),
            username,
            avatar_url,
        })
        .collect();

    Ok(Json(results))
}
