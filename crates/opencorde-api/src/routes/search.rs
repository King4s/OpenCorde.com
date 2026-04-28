//! # Route: Full-Text Search
//! Full-text message search using Tantivy.
//!
//! ## Endpoints
//! - GET /api/v1/search?q=query&server_id=X&channel_id=Y&limit=20
//!
//! ## Query Parameters
//! - `q` (required) — Search query string (min 1 char)
//! - `server_id` (optional) — Filter results by server ID
//! - `channel_id` (optional) — Filter results by channel ID
//! - `limit` (optional) — Max results to return (default 20, max 100)
//!
//! ## Requires Authentication
//! Bearer token required in Authorization header.
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_search (search engine)
//! - crate::AppState (application state)
//! - crate::middleware::auth::AuthUser (authentication)

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::AppState;
use crate::error::ApiError;
use crate::middleware::auth::AuthUser;
use crate::routes::permission_check;
use opencorde_core::{Snowflake, permissions::Permissions};

/// Search query parameters.
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    /// Search query string
    q: String,
    /// Optional server ID filter
    server_id: Option<u64>,
    /// Optional channel ID filter
    channel_id: Option<u64>,
    /// Max results (default 20, max 100)
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    20
}

/// Search result wrapper (re-exported from opencorde_search).
#[derive(Debug, Serialize)]
pub struct SearchResult {
    /// Unique message identifier
    pub message_id: u64,
    /// Channel containing this message
    pub channel_id: u64,
    /// Server containing the channel
    pub server_id: u64,
    /// Author of the message
    pub author_id: u64,
    /// Message text content
    pub content: String,
    /// Relevance score (higher = better match)
    pub score: f32,
}

/// Search response body.
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    /// Search results sorted by relevance
    pub results: Vec<SearchResult>,
    /// Total number of results
    pub count: usize,
}

/// Create the search router.
pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/search", get(search_messages))
}

/// Full-text message search endpoint.
///
/// Searches messages by content query with optional server/channel filtering.
/// Requires authentication.
///
/// # Errors
/// - `400` — Invalid query parameters or empty query string
/// - `401` — Missing or invalid authorization
/// - `503` — Search engine not available
#[instrument(skip(state), fields(user_id = %auth.user_id))]
async fn search_messages(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<SearchResponse>, ApiError> {
    tracing::info!(
        query = %params.q,
        limit = params.limit,
        ?params.server_id,
        ?params.channel_id,
        "search request"
    );

    // Validate query
    if params.q.trim().is_empty() {
        tracing::warn!("empty search query");
        return Err(ApiError::BadRequest(
            "search query cannot be empty".to_string(),
        ));
    }

    // Validate limit (clamp between 1 and 100)
    let limit = params.limit.clamp(1, 100);

    // Get search engine from state
    let search_engine = match &state.search {
        Some(engine) => engine,
        None => {
            tracing::warn!("search engine not available");
            return Err(ApiError::ServiceUnavailable(
                "search engine not available".to_string(),
            ));
        }
    };

    // Execute search. Fetch extra hits because inaccessible channels are filtered below.
    let search_limit = limit.saturating_mul(5).clamp(limit, 100);
    let results = search_engine
        .search(&params.q, params.server_id, params.channel_id, search_limit)
        .map_err(|e| {
            tracing::error!(error = %e, "search execution failed");
            ApiError::InternalServerError(format!("search failed: {}", e))
        })?;

    let mut filtered = Vec::with_capacity(limit.min(results.len()));
    for r in results {
        let Ok(channel_id) = i64::try_from(r.channel_id) else {
            tracing::warn!(
                channel_id = r.channel_id,
                "search result has invalid channel id"
            );
            continue;
        };

        match permission_check::require_channel_perm(
            &state.db,
            auth.user_id,
            Snowflake::new(channel_id),
            Permissions::VIEW_CHANNEL,
        )
        .await
        {
            Ok(()) => {
                filtered.push(SearchResult {
                    message_id: r.message_id,
                    channel_id: r.channel_id,
                    server_id: r.server_id,
                    author_id: r.author_id,
                    content: r.content,
                    score: r.score,
                });
                if filtered.len() >= limit {
                    break;
                }
            }
            Err(ApiError::Forbidden | ApiError::NotFound(_)) => {
                tracing::debug!(
                    channel_id = r.channel_id,
                    message_id = r.message_id,
                    "filtered unauthorized search result"
                );
            }
            Err(err) => return Err(err),
        }
    }

    let count = filtered.len();

    tracing::info!(results = count, "search completed successfully");

    Ok(Json(SearchResponse {
        results: filtered,
        count,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_params_parsing() {
        let json = r#"{"q": "hello", "server_id": 123, "channel_id": 456, "limit": 50}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.q, "hello");
        assert_eq!(params.server_id, Some(123));
        assert_eq!(params.channel_id, Some(456));
        assert_eq!(params.limit, 50);
    }

    #[test]
    fn test_search_params_default_limit() {
        let json = r#"{"q": "hello"}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.limit, 20);
    }

    #[test]
    fn test_search_response_serialization() {
        let response = SearchResponse {
            results: vec![SearchResult {
                message_id: 1001,
                channel_id: 2001,
                server_id: 3001,
                author_id: 4001,
                content: "hello".to_string(),
                score: 0.95,
            }],
            count: 1,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("hello"));
        assert!(json.contains("0.95"));
    }
}
