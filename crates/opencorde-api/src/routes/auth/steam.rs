//! # Steam OpenID 2.0 Authentication
//! Steam login endpoints using OpenID 2.0 protocol.
//!
//! ## Endpoints
//! - GET /api/v1/auth/steam — Redirect to Steam OpenID login
//! - GET /api/v1/auth/steam/callback — Verify Steam response and login/register user
//!
//! ## Flow
//! 1. User clicks "Sign in with Steam" → GET /api/v1/auth/steam
//! 2. Server redirects to https://steamcommunity.com/openid/login?{params}
//! 3. Steam redirects back to /api/v1/auth/steam/callback with signed identity
//! 4. Server verifies signature by POSTing back to Steam's endpoint
//! 5. Extract Steam64 ID from openid.claimed_id URL
//! 6. Find or create user, generate JWT tokens
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db::repos::user_repo (user CRUD)
//! - opencorde_core::Snowflake (ID generation)
//! - crate::jwt (token creation)
//! - crate::AppState (database + config)
//! - crate::error::ApiError (unified error handling)
//! - super::steam_verify (verification helpers)

use anyhow::anyhow;
use axum::{
    extract::{Query, State},
    response::Redirect,
};
use opencorde_core::Snowflake;
use opencorde_db::repos::user_repo;
use serde::Deserialize;

use crate::{jwt, AppState, error::ApiError};
use super::steam_verify;

/// OpenID 2.0 callback query parameters from Steam.
#[derive(Debug, Deserialize)]
pub struct SteamCallbackParams {
    #[serde(rename = "openid.ns")]
    pub openid_ns: Option<String>,
    #[serde(rename = "openid.mode")]
    pub openid_mode: Option<String>,
    #[serde(rename = "openid.op_endpoint")]
    pub openid_op_endpoint: Option<String>,
    #[serde(rename = "openid.claimed_id")]
    pub openid_claimed_id: Option<String>,
    #[serde(rename = "openid.identity")]
    pub openid_identity: Option<String>,
    #[serde(rename = "openid.return_to")]
    pub openid_return_to: Option<String>,
    #[serde(rename = "openid.response_nonce")]
    pub openid_response_nonce: Option<String>,
    #[serde(rename = "openid.assoc_handle")]
    pub openid_assoc_handle: Option<String>,
    #[serde(rename = "openid.signed")]
    pub openid_signed: Option<String>,
    #[serde(rename = "openid.sig")]
    pub openid_sig: Option<String>,
}

/// GET /api/v1/auth/steam — Redirect to Steam OpenID login endpoint.
#[tracing::instrument(skip(state))]
pub async fn steam_login(State(state): State<AppState>) -> Redirect {
    tracing::info!("steam login initiated");

    let return_to = format!("{}/api/v1/auth/steam/callback", state.config.base_url);
    let realm = state.config.base_url.clone();

    let params = vec![
        ("openid.ns", "http://specs.openid.net/auth/2.0"),
        ("openid.mode", "checkid_setup"),
        ("openid.return_to", &return_to),
        ("openid.realm", &realm),
        ("openid.identity", "http://specs.openid.net/auth/2.0/identifier_select"),
        ("openid.claimed_id", "http://specs.openid.net/auth/2.0/identifier_select"),
    ];

    let query_string = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    let steam_login_url = format!("https://steamcommunity.com/openid/login?{}", query_string);

    tracing::info!(url = %steam_login_url, "redirecting to steam");

    Redirect::to(&steam_login_url)
}

/// GET /api/v1/auth/steam/callback — Verify Steam response and authenticate user.
#[tracing::instrument(skip(state, params))]
pub async fn steam_callback(
    State(state): State<AppState>,
    Query(params): Query<SteamCallbackParams>,
) -> Result<Redirect, ApiError> {
    tracing::info!("steam callback received");

    // Verify the OpenID response from Steam
    let steam_id = steam_verify::verify_steam_identity(&params).await?;

    tracing::info!(steam_id = %steam_id, "steam identity verified");

    // Check if user exists with this steam_id
    let existing_user = user_repo::get_by_steam_id(&state.db, &steam_id)
        .await
        .map_err(ApiError::Database)?;

    let user_row = match existing_user {
        Some(user) => {
            tracing::info!(user_id = user.id, steam_id = %steam_id, "existing steam user found");
            user
        }
        None => {
            // Create new user from Steam data
            let new_user = steam_verify::create_steam_user(&state, &steam_id).await?;
            tracing::info!(user_id = new_user.id, steam_id = %steam_id, "new steam user created");
            new_user
        }
    };

    // Generate JWT tokens
    let user_id = Snowflake::new(user_row.id);
    let access_token = jwt::create_access_token(
        user_id,
        &user_row.username,
        &state.config.jwt_secret,
        state.config.jwt_access_expiry,
    )
    .map_err(|e| ApiError::Internal(anyhow!("token creation failed: {}", e)))?;

    let refresh_token = jwt::create_refresh_token(
        user_id,
        &user_row.username,
        &state.config.jwt_secret,
        state.config.jwt_refresh_expiry,
    )
    .map_err(|e| ApiError::Internal(anyhow!("token creation failed: {}", e)))?;

    tracing::info!(user_id = user_row.id, "steam login tokens generated");

    // Redirect to login page with tokens as URL encoded query params
    let redirect_url = format!(
        "{}/login?access_token={}&refresh_token={}",
        state.config.base_url,
        urlencoding::encode(&access_token),
        urlencoding::encode(&refresh_token)
    );

    Ok(Redirect::to(&redirect_url))
}
