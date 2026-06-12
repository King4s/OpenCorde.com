//! GET /api/v1/servers/{server_id}/integration-logs
//! List integration logs for a server, with optional application_id filter.
//! Gated on server:VIEW_AUDIT_LOG permission.

use axum::{extract::{Path, Query, State}, Json, Router, routing::get};
use opencorde_core::permissions::Permissions;
use serde::{Deserialize, Serialize};

use crate::routes::helpers::parse_snowflake;
use crate::routes::permission_check;
use crate::{AppState, error::ApiError, middleware::auth::AuthUser};
use opencorde_db::repos::integration_log_repo;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/servers/{server_id}/integration-logs", get(list_logs))
}

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub application_id: Option<String>,
    pub action_type: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct LogEntry {
    pub id: String,
    pub application_id: Option<String>,
    pub actor_bot_user_id: Option<String>,
    pub action_type: String,
    pub status: String,
    pub metadata: serde_json::Value,
    pub created_at: String,
}

impl From<integration_log_repo::IntegrationLogRow> for LogEntry {
    fn from(r: integration_log_repo::IntegrationLogRow) -> Self {
        LogEntry {
            id: r.id.to_string(),
            application_id: r.application_id.map(|id| id.to_string()),
            actor_bot_user_id: r.actor_bot_user_id.map(|id| id.to_string()),
            action_type: r.action_type,
            status: r.status,
            metadata: r.metadata,
            created_at: r.created_at.to_rfc3339(),
        }
    }
}

pub async fn list_logs(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id_str): Path<String>,
    Query(query): Query<LogQuery>,
) -> Result<Json<Vec<LogEntry>>, ApiError> {
    let server_id = parse_snowflake(&server_id_str)?;

    // Require VIEW_AUDIT_LOG permission
    permission_check::require_server_perm(
        &state.db, auth.user_id, server_id, Permissions::VIEW_AUDIT_LOG,
    ).await?;

    let limit = query.limit.unwrap_or(50).min(100);

    let rows = if let Some(ref app_id_str) = query.application_id {
        let app_id = parse_snowflake(app_id_str)?;
        integration_log_repo::list_by_server_and_app(&state.db, server_id, app_id, limit)
            .await
            .map_err(ApiError::Database)?
    } else {
        integration_log_repo::list_by_server(&state.db, server_id, limit)
            .await
            .map_err(ApiError::Database)?
    };

    let entries: Vec<LogEntry> = rows.into_iter().map(LogEntry::from).collect();
    Ok(Json(entries))
}
