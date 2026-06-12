//! Bot token management — create, list, rotate, revoke.

use axum::{extract::{Path, State}, http::StatusCode, Json};
use opencorde_core::snowflake::{Snowflake, SnowflakeGenerator};
use opencorde_core::password::hash_password;
use opencorde_core::models::bot_token::{BotTokenCreated, BotTokenPublic};
use opencorde_db::repos::{app_repo, bot_token_repo};

use crate::routes::helpers::parse_snowflake;
use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

fn gen_token() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let r = RandomState::new();
    let parts: Vec<String> = (0..4).map(|_| format!("{:016x}", r.build_hasher().finish())).collect();
    format!("ocb_{}", parts.join(""))
}

pub async fn create_token(
    State(state): State<AppState>, auth: AuthUser,
    Path(app_id_str): Path<String>,
) -> Result<(StatusCode, Json<BotTokenCreated>), ApiError> {
    let app_id = parse_snowflake(&app_id_str)?;
    let app = app_repo::get_by_id(&state.db, app_id).await.map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("application not found".to_string()))?;
    if app.owner_user_id != auth.user_id.as_i64() { return Err(ApiError::Forbidden); }

    // Find bot user for this app
    let bot_rows = opencorde_db::repos::bot_user_repo::list_by_application(&state.db, app_id)
        .await.map_err(ApiError::Database)?;
    let bot = bot_rows.first().ok_or_else(|| ApiError::BadRequest("application has no bot user".into()))?;

    let mut generator = SnowflakeGenerator::new(11, 0);
    let token_id = generator.next_id();
    let token = gen_token();
    let token_hash = hash_password(&token).map_err(|_| ApiError::InternalServerError("hashing failed".into()))?;
    let prefix = format!("{}...", &token[..8]);

    let row = bot_token_repo::create_token(
        &state.db, token_id, app_id, Snowflake::new(bot.id),
        &prefix, &token_hash, 0, Snowflake::new(auth.user_id.as_i64()), None,
    ).await.map_err(ApiError::Database)?;

    Ok((StatusCode::CREATED, Json(BotTokenCreated {
        id: row.id, token, token_prefix: prefix, intents: row.intents,
        label: row.label, created_at: row.created_at.to_rfc3339(),
        warning: "Copy this token now. It will not be shown again.",
    })))
}

pub async fn list_tokens(
    State(state): State<AppState>, auth: AuthUser,
    Path(app_id_str): Path<String>,
) -> Result<Json<Vec<BotTokenPublic>>, ApiError> {
    let app_id = parse_snowflake(&app_id_str)?;
    let app = app_repo::get_by_id(&state.db, app_id).await.map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("application not found".to_string()))?;
    if app.owner_user_id != auth.user_id.as_i64() { return Err(ApiError::Forbidden); }

    let rows = bot_token_repo::list_tokens(&state.db, app_id).await.map_err(ApiError::Database)?;
    let tokens = rows.into_iter().map(|r| BotTokenPublic {
        id: r.id, bot_user_id: r.bot_user_id,
        token_prefix: r.token_prefix, intents: r.intents, label: r.label,
        last_used_at: r.last_used_at.map(|t| t.to_rfc3339()),
        created_at: r.created_at.to_rfc3339(), revoked: r.revoked_at.is_some(),
    }).collect();
    Ok(Json(tokens))
}

pub async fn revoke_token(
    State(state): State<AppState>, auth: AuthUser,
    Path((app_id_str, token_id_str)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let app_id = parse_snowflake(&app_id_str)?;
    let token_id = parse_snowflake(&token_id_str)?;
    let app = app_repo::get_by_id(&state.db, app_id).await.map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("application not found".to_string()))?;
    if app.owner_user_id != auth.user_id.as_i64() { return Err(ApiError::Forbidden); }

    let ok = bot_token_repo::revoke_token(&state.db, token_id, app_id).await.map_err(ApiError::Database)?;
    if !ok { return Err(ApiError::NotFound("token not found or already revoked".into())); }
    Ok(StatusCode::NO_CONTENT)
}
