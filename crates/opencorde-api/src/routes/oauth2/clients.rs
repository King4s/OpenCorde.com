//! OAuth2 client secret CRUD — create, list, revoke.

use axum::{extract::{Path, State}, http::StatusCode, Json};
use opencorde_core::snowflake::{Snowflake, SnowflakeGenerator};
use opencorde_core::password::hash_password;
use opencorde_core::models::oauth_client_secret::OAuthClientSecretPublic;
use opencorde_db::repos::{app_repo, oauth_client_secret_repo};
use serde::Serialize;

use crate::routes::helpers::parse_snowflake;
use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

#[derive(Debug, Serialize)]
pub struct CreateSecretResponse {
    pub id: i64,
    pub secret: String,
    pub secret_prefix: String,
    pub label: Option<String>,
    pub created_at: String,
    pub warning: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ListSecretsResponse {
    pub secrets: Vec<OAuthClientSecretPublic>,
}

fn generate_secret() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let mut r = RandomState::new();
    let parts: Vec<String> = (0..4).map(|_| {
        let h = r.build_hasher().finish();
        format!("{:016x}", h)
    }).collect();
    format!("ocs_{}", parts.join(""))
}

pub async fn create_secret(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(app_id_str): Path<String>,
) -> Result<(StatusCode, Json<CreateSecretResponse>), ApiError> {
    let app_id = parse_snowflake(&app_id_str)?;
    let app = app_repo::get_by_id(&state.db, app_id).await.map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("application not found".to_string()))?;
    if app.owner_user_id != auth.user_id.as_i64() {
        return Err(ApiError::Forbidden);
    }

    let mut generator = SnowflakeGenerator::new(9, 0);
    let secret_id = generator.next_id();
    let secret = generate_secret();
    let secret_hash = hash_password(&secret).map_err(|_| ApiError::InternalServerError("hashing failed".into()))?;
    let prefix = format!("{}...", &secret[..8]);

    let row = oauth_client_secret_repo::create_secret(
        &state.db, secret_id, app_id, &prefix, &secret_hash, Snowflake::new(auth.user_id.as_i64()), None,
    ).await.map_err(ApiError::Database)?;

    Ok((StatusCode::CREATED, Json(CreateSecretResponse {
        id: row.id, secret, secret_prefix: prefix, label: row.label,
        created_at: row.created_at.to_rfc3339(),
        warning: "Copy this secret now. It will not be shown again.",
    })))
}

pub async fn list_secrets(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(app_id_str): Path<String>,
) -> Result<Json<ListSecretsResponse>, ApiError> {
    let app_id = parse_snowflake(&app_id_str)?;
    let app = app_repo::get_by_id(&state.db, app_id).await.map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("application not found".to_string()))?;
    if app.owner_user_id != auth.user_id.as_i64() { return Err(ApiError::Forbidden); }

    let rows = oauth_client_secret_repo::list_secrets(&state.db, app_id).await.map_err(ApiError::Database)?;
    let secrets = rows.into_iter().map(|r| OAuthClientSecretPublic {
        id: r.id, application_id: r.application_id,
        secret_prefix: r.secret_prefix, label: r.label,
        created_at: r.created_at.to_rfc3339(), revoked: r.revoked_at.is_some(),
    }).collect();
    Ok(Json(ListSecretsResponse { secrets }))
}

pub async fn revoke_secret(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((app_id_str, secret_id_str)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let app_id = parse_snowflake(&app_id_str)?;
    let secret_id = parse_snowflake(&secret_id_str)?;
    let app = app_repo::get_by_id(&state.db, app_id).await.map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("application not found".to_string()))?;
    if app.owner_user_id != auth.user_id.as_i64() { return Err(ApiError::Forbidden); }

    let revoked = oauth_client_secret_repo::revoke_secret(&state.db, secret_id, app_id)
        .await.map_err(ApiError::Database)?;
    if !revoked { return Err(ApiError::NotFound("secret not found or already revoked".into())); }
    Ok(StatusCode::NO_CONTENT)
}
