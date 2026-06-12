//! OAuth2 token exchange — authorization-code and refresh-token grants.
//!
//! ## Supported grant types
//! - `authorization_code` — exchange a one-time authorization code for tokens
//! - `refresh_token` — rotate a refresh token for a new token pair
//!
//! ## Revocation
//! `POST /oauth2/revoke` revokes a refresh token (JTI lookup + mark revoked).

use axum::Json;
use axum::extract::State;
use chrono::{Duration, Utc};
use opencorde_core::snowflake::Snowflake;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::error::ApiError;
use crate::jwt;
use crate::AppState;
use opencorde_db::repos;

// ── Request / Response types ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub grant_type: String,
    pub redirect_uri: Option<String>,
    /// When grant_type=refresh_token, this carries the refresh token.
    #[serde(default)]
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u32,
    pub refresh_token: String,
    pub scope: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RevokeRequest {
    pub client_id: String,
    pub client_secret: String,
    pub token: String,
    #[serde(default)]
    pub token_type_hint: String,
}

// ── Helpers ───────────────────────────────────────────────────────────

/// Parse an i64 client_id string (which is a Snowflake) from the form body.
fn parse_client_id(raw: &str) -> Result<Snowflake, ApiError> {
    let id: i64 = raw
        .parse()
        .map_err(|_| ApiError::BadRequest("invalid client_id".into()))?;
    Ok(Snowflake::new(id))
}

/// Verify client credentials against stored Argon2id hashes.
async fn verify_client(
    db: &sqlx::PgPool,
    application_id: Snowflake,
    client_secret: &str,
) -> Result<bool, ApiError> {
    repos::oauth_client_secret_repo::verify_client_secret(db, application_id, client_secret)
        .await
        .map_err(ApiError::Database)
}

// ── Handlers ──────────────────────────────────────────────────────────

/// POST /api/v1/oauth2/token
///
/// Supports `grant_type=authorization_code` and `grant_type=refresh_token`.
#[instrument(skip(state), fields(grant_type = %req.grant_type))]
pub async fn exchange_token(
    State(state): State<AppState>,
    axum::extract::Form(req): axum::extract::Form<TokenRequest>,
) -> Result<Json<TokenResponse>, ApiError> {
    let client_id = parse_client_id(&req.client_id)?;

    // 1. Verify client credentials
    let valid = verify_client(&state.db, client_id, &req.client_secret).await?;
    if !valid {
        return Err(ApiError::BadRequest("invalid_client".into()));
    }

    match req.grant_type.as_str() {
        "authorization_code" => handle_authorization_code(&state, client_id, &req).await,
        "refresh_token" => handle_refresh_token(&state, client_id, &req).await,
        other => Err(ApiError::BadRequest(format!(
            "unsupported grant_type: {other}"
        ))),
    }
}

/// Exchange a one-time authorization code for an access token + refresh token.
async fn handle_authorization_code(
    state: &AppState,
    application_id: Snowflake,
    req: &TokenRequest,
) -> Result<Json<TokenResponse>, ApiError> {
    let redirect_uri = req
        .redirect_uri
        .as_deref()
        .unwrap_or("");

    // 2. Validate + consume the authorization code
    let code_row = repos::oauth_auth_code_repo::consume_auth_code(
        &state.db,
        application_id,
        &req.code,
        redirect_uri,
    )
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::BadRequest("invalid_grant".into()))?;

    let user_id = Snowflake::new(code_row.user_id);

    // 3. Look up username for the token
    let user_row = repos::user_repo::get_by_id(&state.db, user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InternalServerError("user not found".into()))?;

    // 4. Issue tokens
    let scope = Some(code_row.scope.as_str());
    let (access_token, refresh_token, _jti) = issue_token_pair(state, user_id, &user_row.username, scope).await?;

    let expires_in = state.config.jwt_access_expiry as u32;

    Ok(Json(TokenResponse {
        access_token,
        token_type: "Bearer".into(),
        expires_in,
        refresh_token,
        scope: code_row.scope,
    }))
}

/// Rotate a refresh token for a new token pair.
async fn handle_refresh_token(
    state: &AppState,
    _application_id: Snowflake,
    req: &TokenRequest,
) -> Result<Json<TokenResponse>, ApiError> {
    // 2. Validate the refresh token JWT
    let claims = jwt::validate_refresh_token(&req.refresh_token, &state.config.jwt_secret)
        .map_err(|_| ApiError::BadRequest("invalid_grant".into()))?;

    let user_id_i64: i64 = claims
        .sub
        .parse()
        .map_err(|_| ApiError::InternalServerError("invalid sub claim".into()))?;
    let user_id = Snowflake::new(user_id_i64);

    // 3. Check JTI theft detection
    let jti = claims
        .jti
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("invalid_grant".into()))?;

    let row = repos::refresh_token_repo::get_by_jti(&state.db, jti)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::BadRequest("invalid_grant".into()))?;

    if row.revoked {
        // Theft detected — revoke ALL user tokens
        repos::refresh_token_repo::revoke_all_for_user(&state.db, user_id_i64)
            .await
            .map_err(ApiError::Database)?;
        return Err(ApiError::BadRequest("invalid_grant".into()));
    }

    // 4. Revoke the old JTI (rotation)
    repos::refresh_token_repo::revoke(&state.db, jti)
        .await
        .map_err(ApiError::Database)?;

    // 5. Look up username
    let user_row = repos::user_repo::get_by_id(&state.db, user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InternalServerError("user not found".into()))?;

    // 6. Issue new token pair
    let scope = claims.scope.as_deref();
    let (access_token, refresh_token, _new_jti) = issue_token_pair(state, user_id, &user_row.username, scope).await?;

    let expires_in = state.config.jwt_access_expiry as u32;

    Ok(Json(TokenResponse {
        access_token,
        token_type: "Bearer".into(),
        expires_in,
        refresh_token,
        scope: claims.scope.unwrap_or_default(),
    }))
}

/// POST /api/v1/oauth2/revoke
///
/// Revokes a refresh token. The client must authenticate with its credentials.
#[instrument(skip(state))]
pub async fn revoke_token(
    State(state): State<AppState>,
    axum::extract::Form(req): axum::extract::Form<RevokeRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let client_id = parse_client_id(&req.client_id)?;

    let valid = verify_client(&state.db, client_id, &req.client_secret).await?;
    if !valid {
        return Err(ApiError::BadRequest("invalid_client".into()));
    }

    // Try to decode the token to extract the JTI
    if let Ok(claims) = jwt::validate_refresh_token(&req.token, &state.config.jwt_secret) {
        if let Some(jti) = &claims.jti {
            repos::refresh_token_repo::revoke(&state.db, jti)
                .await
                .map_err(ApiError::Database)?;
        }
    }

    Ok(Json(serde_json::json!({ "revoked": true })))
}

// ── Internal ──────────────────────────────────────────────────────────

/// Issue an access token + refresh token pair and persist the refresh token JTI.
/// Returns `(access_token, refresh_token, jti)`.
async fn issue_token_pair(
    state: &AppState,
    user_id: Snowflake,
    username: &str,
    scope: Option<&str>,
) -> Result<(String, String, String), ApiError> {
    let access_token = jwt::create_access_token(
        user_id,
        username,
        &state.config.jwt_secret,
        state.config.jwt_access_expiry,
        scope,
    )
    .map_err(|e| ApiError::InternalServerError(format!("token creation failed: {e}")))?;

    let (refresh_token, jti) = jwt::create_refresh_token(
        user_id,
        username,
        &state.config.jwt_secret,
        state.config.jwt_refresh_expiry,
        scope,
    )
    .map_err(|e| ApiError::InternalServerError(format!("token creation failed: {e}")))?;

    // Persist the refresh token JTI
    let expires_at = Utc::now() + Duration::seconds(state.config.jwt_refresh_expiry as i64);
    repos::refresh_token_repo::insert(&state.db, &jti, user_id.as_i64(), expires_at)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("failed to store refresh JTI: {e}")))?;

    Ok((access_token, refresh_token, jti))
}
