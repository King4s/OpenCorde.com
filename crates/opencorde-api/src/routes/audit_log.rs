//! # Route: Audit Log
//! Server audit log for moderation history.
//!
//! ## Endpoints
//! - GET /api/v1/servers/{id}/audit-log — List recent actions (server owner only)
//!
//! ## Depends On
//! - opencorde_db::repos::audit_repo
//! - crate::AppState

use crate::{AppState, error::ApiError, middleware::auth::AuthUser, routes::{helpers, permission_check}};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use opencorde_db::repos::audit_repo;
use opencorde_core::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::instrument;

#[derive(Debug, Serialize)]
pub struct AuditLogEntry {
    pub id: String,
    pub actor_id: Option<String>,
    pub actor_username: Option<String>,
    pub action: String,
    pub target_id: Option<String>,
    pub target_type: Option<String>,
    pub changes: Option<JsonValue>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    pub before: Option<String>,
}

fn default_limit() -> i64 {
    50
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/api/v1/servers/{id}/audit-log",
        get(list_audit_log),
    )
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn list_audit_log(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<AuditLogEntry>>, ApiError> {
    tracing::info!("fetching audit log");

    let server_id = helpers::parse_snowflake(&server_id)?;
    let limit = query.limit.clamp(1, 100);
    let before_id = query.before.as_ref().map(|b| {
        b.parse::<i64>()
            .map_err(|_| ApiError::BadRequest("invalid before cursor".into()))
    }).transpose()?;

    let _server = opencorde_db::repos::server_repo::get_by_id(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;

    permission_check::require_server_perm(&state.db, auth.user_id, server_id, Permissions::VIEW_AUDIT_LOG).await?;

    let entries = audit_repo::list_entries(&state.db, server_id, limit, before_id)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(count = entries.len(), "audit log entries fetched");

    Ok(Json(
        entries
            .into_iter()
            .map(|e| AuditLogEntry {
                id: e.id.to_string(),
                actor_id: e.actor_id.map(|id| id.to_string()),
                actor_username: e.actor_username,
                action: e.action,
                target_id: e.target_id.map(|id| id.to_string()),
                target_type: e.target_type,
                changes: e.changes,
                created_at: e.created_at,
            })
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limit() {
        let query: ListQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(query.limit, 50);
    }

    #[test]
    fn test_limit_clamping() {
        let limit = 200i64;
        let clamped = limit.max(1).min(100);
        assert_eq!(clamped, 100);
    }
}
