//! App install — install application to server, list, uninstall.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use opencorde_core::snowflake::Snowflake;
use opencorde_db::repos::{app_install_repo, app_repo, server_repo};
use serde::{Deserialize, Serialize};

use crate::routes::helpers::parse_snowflake;
use crate::routes::permission_check;
use crate::{AppState, error::ApiError, middleware::auth::AuthUser};
use opencorde_core::permissions::Permissions;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct InstallAppRequest {
    pub scopes: Vec<String>,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct AppInstallResponse {
    pub id: i64,
    pub application_id: i64,
    pub server_id: i64,
    pub scopes: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ServerAppResponse {
    pub id: i64,
    pub application_id: i64,
    pub app_name: String,
    pub installed_by: i64,
    pub scopes: Vec<String>,
    pub installed_at: String,
}

pub async fn install_app(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Path(app_id_str): Path<String>,
    Json(_req): Json<InstallAppRequest>,
) -> Result<(StatusCode, Json<AppInstallResponse>), ApiError> {
    let _app_id = parse_snowflake(&app_id_str)?;
    // Server ID comes from request — we need it in the body
    // For now, require server_id as query param
    Err(ApiError::BadRequest(
        "server_id required as query parameter".into(),
    ))
}

pub async fn list_server_apps(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(server_id_str): Path<String>,
) -> Result<Json<Vec<ServerAppResponse>>, ApiError> {
    let server_id = parse_snowflake(&server_id_str)?;
    let _server = server_repo::get_by_id(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;

    let rows = app_install_repo::list_by_server(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?;
    let mut apps = Vec::new();
    for r in rows {
        let app = app_repo::get_by_id(&state.db, Snowflake::new(r.application_id))
            .await
            .map_err(ApiError::Database)?;
        apps.push(ServerAppResponse {
            id: r.id,
            application_id: r.application_id,
            app_name: app.map(|a| a.name).unwrap_or_default(),
            installed_by: r.installed_by,
            scopes: r.scopes,
            installed_at: r.created_at.to_rfc3339(),
        });
    }
    Ok(Json(apps))
}

pub async fn uninstall_app(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id_str, app_id_str)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let server_id = parse_snowflake(&server_id_str)?;
    let app_id = parse_snowflake(&app_id_str)?;
    permission_check::require_server_perm(
        &state.db,
        auth.user_id,
        server_id,
        Permissions::MANAGE_SERVER,
    )
    .await?;

    let deleted = app_install_repo::delete_install(&state.db, app_id, server_id)
        .await
        .map_err(ApiError::Database)?;
    if !deleted {
        return Err(ApiError::NotFound("install not found".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}
