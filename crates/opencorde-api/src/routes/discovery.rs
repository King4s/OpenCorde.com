//! # Route: Server Discovery
//! Public server discovery listing (no auth required).
//!
//! ## Endpoints
//! - GET /api/v1/discover — List public servers (no auth)
//! - PATCH /api/v1/servers/{id}/discovery — Toggle public listing (owner auth)
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db::repos::server_repo
//! - opencorde_core::Snowflake
//! - crate::middleware::auth::AuthUser
//! - crate::AppState

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, patch},
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use crate::{AppState, error::ApiError, middleware::auth::AuthUser};
use crate::routes::helpers::parse_snowflake;

/// Public server info for discovery listing.
#[derive(Debug, Serialize, Clone)]
pub struct DiscoveryServer {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub member_count: i32,
    pub tags: Option<String>,
}

/// Request to update server discovery settings.
#[derive(Debug, Deserialize)]
pub struct DiscoveryUpdateRequest {
    pub public: bool,
    pub description: Option<String>,
    pub tags: Option<String>,
}

/// Query parameters for discovery listing.
#[derive(Debug, Deserialize)]
pub struct DiscoveryQuery {
    /// Optional search query (name, description, or tags)
    pub q: Option<String>,
    /// Number of results to return (default 20, max 50)
    pub limit: Option<i32>,
}

/// Build the discovery router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/discover", get(list_discovery))
        .route("/api/v1/servers/{id}/discovery", patch(update_discovery))
}

/// GET /api/v1/discover — List public servers (no auth required).
///
/// Returns paginated list of public servers, optionally filtered by search query.
#[instrument(skip(state))]
async fn list_discovery(
    State(state): State<AppState>,
    Query(params): Query<DiscoveryQuery>,
) -> Result<Json<Vec<DiscoveryServer>>, ApiError> {
    let limit = params.limit.unwrap_or(20).clamp(1, 50);

    tracing::info!(limit = limit, query = ?params.q, "fetching public servers");

    let servers = if let Some(search) = params.q.as_ref() {
        let query = format!("%{}%", search);
        sqlx::query_as::<_, (i64, String, Option<String>, Option<String>, i32, Option<String>)>(
            "SELECT id, name, description, icon_url, member_count, tags FROM servers \
             WHERE public = TRUE AND (name ILIKE $1 OR description ILIKE $1 OR tags ILIKE $1) \
             ORDER BY member_count DESC LIMIT $2"
        )
        .bind(query)
        .bind(limit as i64)
        .fetch_all(&state.db)
        .await
        .map_err(ApiError::Database)?
    } else {
        sqlx::query_as::<_, (i64, String, Option<String>, Option<String>, i32, Option<String>)>(
            "SELECT id, name, description, icon_url, member_count, tags FROM servers \
             WHERE public = TRUE ORDER BY member_count DESC LIMIT $1"
        )
        .bind(limit as i64)
        .fetch_all(&state.db)
        .await
        .map_err(ApiError::Database)?
    };

    let responses: Vec<DiscoveryServer> = servers
        .into_iter()
        .map(|(id, name, description, icon_url, member_count, tags)| DiscoveryServer {
            id: id.to_string(),
            name,
            description,
            icon_url,
            member_count,
            tags,
        })
        .collect();

    tracing::info!(count = responses.len(), "discovery servers fetched");

    Ok(Json(responses))
}

/// PATCH /api/v1/servers/{id}/discovery — Update discovery settings (owner only).
///
/// Requires authentication. User must be server owner.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn update_discovery(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<DiscoveryUpdateRequest>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(server_id = %id, "updating discovery settings");

    // Parse server ID
    let server_id_sf = parse_snowflake(&id)?;
    let server_id = server_id_sf.as_i64();

    // Fetch server and verify ownership
    let server = sqlx::query_as::<_, (i64,)>(
        "SELECT owner_id FROM servers WHERE id = $1"
    )
    .bind(server_id)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| {
        tracing::warn!(server_id = server_id, "server not found");
        ApiError::NotFound("server not found".into())
    })?;

    let owner_id = server.0;
    let user_id = auth.user_id.as_i64();

    if owner_id != user_id {
        tracing::warn!(server_id = server_id, "user not server owner");
        return Err(ApiError::Forbidden);
    }

    // Update discovery settings
    sqlx::query(
        "UPDATE servers SET public = $1, description = COALESCE($2, description), \
         tags = COALESCE($3, tags), updated_at = NOW() WHERE id = $4"
    )
    .bind(req.public)
    .bind(req.description)
    .bind(req.tags)
    .bind(server_id)
    .execute(&state.db)
    .await
    .map_err(ApiError::Database)?;

    tracing::info!(
        server_id = server_id,
        public = req.public,
        "discovery settings updated"
    );

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_query_deserialization() {
        let json = r#"{"q":"test","limit":30}"#;
        let query: DiscoveryQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.q, Some("test".to_string()));
        assert_eq!(query.limit, Some(30));
    }

    #[test]
    fn test_discovery_server_serialization() {
        let server = DiscoveryServer {
            id: "123456".to_string(),
            name: "Test Server".to_string(),
            description: Some("A test server".to_string()),
            icon_url: None,
            member_count: 42,
            tags: Some("gaming,community".to_string()),
        };

        let json = serde_json::to_string(&server).unwrap();
        assert!(json.contains("Test Server"));
        assert!(json.contains("42"));
    }
}
