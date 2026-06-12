//! Application commands — create, list, delete for apps.

use axum::{extract::{Path, State}, http::StatusCode, Json};
use opencorde_core::snowflake::SnowflakeGenerator;
use opencorde_core::models::application_command::ApplicationCommandResponse;
use opencorde_db::repos::{app_repo, app_command_repo};
use serde::Deserialize;

use crate::routes::helpers::parse_snowflake;
use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

#[derive(Debug, Deserialize)]
pub struct CreateCommandRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_type")]
    pub command_type: i16,
    #[serde(default)]
    pub options: serde_json::Value,
}

fn default_type() -> i16 { 1 }

pub async fn create_command(
    State(state): State<AppState>, auth: AuthUser,
    Path(app_id_str): Path<String>,
    Json(req): Json<CreateCommandRequest>,
) -> Result<(StatusCode, Json<ApplicationCommandResponse>), ApiError> {
    let app_id = parse_snowflake(&app_id_str)?;
    let app = app_repo::get_by_id(&state.db, app_id).await.map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("application not found".to_string()))?;
    if app.owner_user_id != auth.user_id.as_i64() { return Err(ApiError::Forbidden); }

    let mut generator = SnowflakeGenerator::new(13, 0);
    let cmd_id = generator.next_id();

    let row = app_command_repo::create_command(
        &state.db, cmd_id, app_id, None, &req.name, &req.description,
        req.command_type, &req.options,
    ).await.map_err(|e| {
        if e.to_string().contains("duplicate key") {
            ApiError::BadRequest(format!("command '{}' already exists", req.name))
        } else { ApiError::Database(e) }
    })?;

    Ok((StatusCode::CREATED, Json(ApplicationCommandResponse {
        id: row.id, application_id: row.application_id,
        server_id: row.server_id, name: row.name, description: row.description,
        command_type: row.command_type, options: row.options, version: row.version,
    })))
}

pub async fn list_commands(
    State(state): State<AppState>, _auth: AuthUser,
    Path(app_id_str): Path<String>,
) -> Result<Json<Vec<ApplicationCommandResponse>>, ApiError> {
    let app_id = parse_snowflake(&app_id_str)?;
    let _app = app_repo::get_by_id(&state.db, app_id).await.map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("application not found".to_string()))?;

    let rows = app_command_repo::list_commands(&state.db, app_id, None).await.map_err(ApiError::Database)?;
    let cmds = rows.into_iter().map(|r| ApplicationCommandResponse {
        id: r.id, application_id: r.application_id,
        server_id: r.server_id, name: r.name, description: r.description,
        command_type: r.command_type, options: r.options, version: r.version,
    }).collect();
    Ok(Json(cmds))
}

pub async fn delete_command(
    State(state): State<AppState>, auth: AuthUser,
    Path((app_id_str, cmd_id_str)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let app_id = parse_snowflake(&app_id_str)?;
    let cmd_id = parse_snowflake(&cmd_id_str)?;
    let app = app_repo::get_by_id(&state.db, app_id).await.map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("application not found".to_string()))?;
    if app.owner_user_id != auth.user_id.as_i64() { return Err(ApiError::Forbidden); }

    let ok = app_command_repo::delete_command(&state.db, cmd_id, app_id).await.map_err(ApiError::Database)?;
    if !ok { return Err(ApiError::NotFound("command not found".into())); }
    Ok(StatusCode::NO_CONTENT)
}
