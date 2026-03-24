//! # AutoMod Handlers
//! HTTP request handlers for AutoMod endpoints.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{patch, post},
};
use opencorde_core::snowflake::{Snowflake, SnowflakeGenerator};
use opencorde_db::repos::{automod_repo, server_repo};
use tracing::instrument;

use crate::{
    error::ApiError, middleware::auth::AuthUser, AppState,
};
use super::super::helpers::parse_snowflake;
use super::types::{AutomodRuleResponse, CreateAutomodRuleRequest, UpdateAutomodRuleRequest};

fn row_to_response(row: automod_repo::AutomodRuleRow) -> AutomodRuleResponse {
    let keywords = row
        .keywords
        .split(',')
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .collect();

    AutomodRuleResponse {
        id: row.id.to_string(),
        server_id: row.server_id.to_string(),
        name: row.name,
        keywords,
        enabled: row.enabled,
        action: row.action,
        created_at: row.created_at,
    }
}

/// POST /api/v1/servers/{server_id}/automod — Create a new AutoMod rule.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn create_rule(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
    Json(req): Json<CreateAutomodRuleRequest>,
) -> Result<(StatusCode, Json<AutomodRuleResponse>), ApiError> {
    tracing::info!("creating automod rule");

    req.validate()
        .map_err(|e| ApiError::BadRequest(e))?;

    // Parse server ID
    let server_id_sf = parse_snowflake(&server_id)?;

    // Verify user is server owner
    let server = server_repo::get_by_id(&state.db, server_id_sf)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch server");
            ApiError::Database(e)
        })?
        .ok_or(ApiError::NotFound("server not found".to_string()))?;

    if server.owner_id != auth.user_id.as_i64() {
        tracing::warn!("user is not server owner");
        return Err(ApiError::Forbidden);
    }

    // Validate name and action
    let name = req.name.unwrap_or_else(|| "Keyword Filter".to_string());
    if name.is_empty() || name.len() > 100 {
        return Err(ApiError::BadRequest(
            "rule name must be 1-100 characters".to_string(),
        ));
    }

    let action = req.action.unwrap_or_else(|| "delete".to_string());
    if action != "delete" && action != "timeout" {
        return Err(ApiError::BadRequest(
            "action must be 'delete' or 'timeout'".to_string(),
        ));
    }

    // Generate rule ID
    let mut generator = SnowflakeGenerator::new(3, 0);
    let rule_id = generator.next_id();

    // Join keywords with comma
    let keywords_str = req.keywords.join(",");

    // Create rule
    let row = automod_repo::create_rule(
        &state.db,
        rule_id,
        server_id_sf,
        &name,
        &keywords_str,
        &action,
        auth.user_id,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to create automod rule");
        ApiError::Database(e)
    })?;

    tracing::info!(rule_id = row.id, "automod rule created successfully");

    Ok((StatusCode::CREATED, Json(row_to_response(row))))
}

/// GET /api/v1/servers/{server_id}/automod — List all AutoMod rules for a server.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn list_rules(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
) -> Result<Json<Vec<AutomodRuleResponse>>, ApiError> {
    tracing::info!("listing automod rules");

    // Parse server ID
    let server_id_sf = parse_snowflake(&server_id)?;

    // Verify server exists (no permission check needed for viewing rules)
    let _server = server_repo::get_by_id(&state.db, server_id_sf)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch server");
            ApiError::Database(e)
        })?
        .ok_or(ApiError::NotFound("server not found".to_string()))?;

    // Fetch rules
    let rows = automod_repo::list_by_server(&state.db, server_id_sf)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to list automod rules");
            ApiError::Database(e)
        })?;

    let rules: Vec<AutomodRuleResponse> = rows.into_iter().map(row_to_response).collect();

    tracing::info!(count = rules.len(), "automod rules fetched successfully");

    Ok(Json(rules))
}

/// PATCH /api/v1/automod/{rule_id} — Update an AutoMod rule.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn update_rule(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(rule_id): Path<String>,
    Json(req): Json<UpdateAutomodRuleRequest>,
) -> Result<Json<AutomodRuleResponse>, ApiError> {
    tracing::info!("updating automod rule");

    req.validate_keywords()
        .map_err(|e| ApiError::BadRequest(e))?;

    // Parse rule ID
    let rule_id_sf = parse_snowflake(&rule_id)?;

    // Fetch current rule to verify ownership and get current values
    let current_row: automod_repo::AutomodRuleRow = sqlx::query_as(
        "SELECT * FROM automod_rules WHERE id = $1",
    )
    .bind(rule_id_sf.as_i64())
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to fetch automod rule");
        ApiError::Database(e)
    })?
    .ok_or(ApiError::NotFound("rule not found".to_string()))?;

    // Verify user is rule creator or server owner
    let server = server_repo::get_by_id(&state.db, Snowflake::new(current_row.server_id))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch server");
            ApiError::Database(e)
        })?
        .ok_or(ApiError::NotFound("server not found".to_string()))?;

    if current_row.created_by != auth.user_id.as_i64()
        && server.owner_id != auth.user_id.as_i64()
    {
        tracing::warn!("user is not rule creator or server owner");
        return Err(ApiError::Forbidden);
    }

    // Apply patches
    let name = req.name.unwrap_or(current_row.name);
    let enabled = req.enabled.unwrap_or(current_row.enabled);
    let action = req.action.unwrap_or(current_row.action);

    // Use new keywords or keep existing
    let keywords = req
        .keywords
        .map(|kw| kw.join(","))
        .unwrap_or(current_row.keywords);

    // Update rule
    automod_repo::update_rule(&state.db, rule_id_sf, &name, &keywords, enabled, &action)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to update automod rule");
            ApiError::Database(e)
        })?;

    // Fetch updated rule
    let updated_row: automod_repo::AutomodRuleRow = sqlx::query_as(
        "SELECT * FROM automod_rules WHERE id = $1",
    )
    .bind(rule_id_sf.as_i64())
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to fetch updated automod rule");
        ApiError::Database(e)
    })?;

    tracing::info!(rule_id = updated_row.id, "automod rule updated successfully");

    Ok(Json(row_to_response(updated_row)))
}

/// DELETE /api/v1/automod/{rule_id} — Delete an AutoMod rule.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn delete_rule(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(rule_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("deleting automod rule");

    // Parse rule ID
    let rule_id_sf = parse_snowflake(&rule_id)?;

    // Fetch rule to verify ownership
    let rule: automod_repo::AutomodRuleRow = sqlx::query_as(
        "SELECT * FROM automod_rules WHERE id = $1",
    )
    .bind(rule_id_sf.as_i64())
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to fetch automod rule");
        ApiError::Database(e)
    })?
    .ok_or(ApiError::NotFound("rule not found".to_string()))?;

    // Verify user is rule creator or server owner
    let server = server_repo::get_by_id(&state.db, Snowflake::new(rule.server_id))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch server");
            ApiError::Database(e)
        })?
        .ok_or(ApiError::NotFound("server not found".to_string()))?;

    if rule.created_by != auth.user_id.as_i64() && server.owner_id != auth.user_id.as_i64() {
        tracing::warn!("user is not rule creator or server owner");
        return Err(ApiError::Forbidden);
    }

    // Delete rule
    automod_repo::delete_rule(&state.db, rule_id_sf)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to delete automod rule");
            ApiError::Database(e)
        })?;

    tracing::info!(rule_id = rule_id_sf.as_i64(), "automod rule deleted successfully");

    Ok(StatusCode::NO_CONTENT)
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/servers/{server_id}/automod",
            post(create_rule).get(list_rules),
        )
        .route(
            "/api/v1/automod/{rule_id}",
            patch(update_rule).delete(delete_rule),
        )
}
