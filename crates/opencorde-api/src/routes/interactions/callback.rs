//! POST /api/v1/interactions/{id}/{token}/callback

use axum::{extract::{Path, State}, http::StatusCode, Json};
use opencorde_core::password::hash_password;
use opencorde_core::snowflake::SnowflakeGenerator;
use opencorde_db::repos::interaction_repo;
use serde::Deserialize;

use crate::routes::helpers::parse_snowflake;
use crate::{AppState, error::ApiError};

#[derive(Debug, Deserialize)]
pub struct InteractionCallback {
    pub r#type: i16,
    #[serde(default)]
    pub data: Option<InteractionCallbackData>,
}

#[derive(Debug, Deserialize)]
pub struct InteractionCallbackData {
    pub content: Option<String>,
    pub embeds: Option<Vec<serde_json::Value>>,
    pub flags: Option<i32>,
}

pub async fn interaction_callback(
    State(state): State<AppState>,
    Path((id_str, token)): Path<(String, String)>,
    Json(body): Json<InteractionCallback>,
) -> Result<StatusCode, ApiError> {
    let interaction_id = parse_snowflake(&id_str)?;
    let token_hash = hash_password(&token).map_err(|_| ApiError::InternalServerError("hash failed".into()))?;

    let _interaction = interaction_repo::find_by_token(&state.db, interaction_id, &token_hash)
        .await.map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("interaction not found or expired".into()))?;

    match body.r#type {
        1 | 4 | 7 => {
            interaction_repo::mark_responded(&state.db, interaction_id, "responded").await.map_err(ApiError::Database)?;
            Ok(StatusCode::OK)
        }
        5 => {
            interaction_repo::mark_responded(&state.db, interaction_id, "deferred").await.map_err(ApiError::Database)?;
            Ok(StatusCode::NO_CONTENT)
        }
        _ => Err(ApiError::BadRequest(format!("unsupported type: {}", body.r#type))),
    }
}
