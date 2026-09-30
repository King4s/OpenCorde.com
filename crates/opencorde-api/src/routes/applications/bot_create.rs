//! POST /api/v1/applications/{id}/bot — create a bot user.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use opencorde_core::models::bot_user::BotUser;
use opencorde_core::snowflake::{Snowflake, SnowflakeGenerator};
use opencorde_db::repos::{app_repo, bot_user_repo};

use super::types::{BotUserResponse, CreateBotRequest};
use crate::routes::helpers::parse_snowflake;
use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn create_bot_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(app_id_str): Path<String>,
    Json(req): Json<CreateBotRequest>,
) -> Result<(StatusCode, Json<BotUserResponse>), ApiError> {
    let app_id = parse_snowflake(&app_id_str)?;

    let app_row = app_repo::get_by_id(&state.db, app_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("application not found".to_string()))?;

    if app_row.owner_user_id != auth.user_id.as_i64() {
        return Err(ApiError::Forbidden);
    }

    let mut generator = SnowflakeGenerator::new(7, 0);
    let bot_id = generator.next_id();

    let bot_row = bot_user_repo::create_bot_user(
        &state.db,
        bot_id,
        app_id,
        req.username.trim(),
        "bot-no-e2ee-key",
        req.avatar_url.as_deref(),
    )
    .await
    .map_err(|e| {
        if e.to_string().contains("duplicate key") {
            ApiError::BadRequest(format!("bot username '{}' already exists", req.username))
        } else {
            ApiError::Database(e)
        }
    })?;

    let bot = BotUser {
        id: Snowflake::new(bot_row.id),
        application_id: Snowflake::new(bot_row.application_id),
        username: bot_row.username,
        avatar_url: bot_row.avatar_url,
        created_at: bot_row.created_at,
    };

    Ok((StatusCode::CREATED, Json(BotUserResponse::from(bot))))
}
