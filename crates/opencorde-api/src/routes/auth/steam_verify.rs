//! # Steam OpenID 2.0 Verification
//! Helper functions for verifying Steam OpenID responses and creating Steam users.
//!
//! ## Depends On
//! - reqwest (HTTP client for Steam API)
//! - opencorde_db::repos::user_repo (user CRUD)
//! - opencorde_core (Snowflake, random)
//! - serde (JSON deserialization)
//! - crate::AppState, crate::error::ApiError

use anyhow::anyhow;
use opencorde_core::SnowflakeGenerator;
use opencorde_db::repos::user_repo;
use serde::Deserialize;
use std::collections::HashMap;

use crate::{AppState, error::ApiError};
use super::steam::SteamCallbackParams;

/// Steam Web API response for GetPlayerSummaries.
#[derive(Debug, Deserialize)]
pub struct SteamApiResponse {
    pub response: SteamApiResponseInner,
}

#[derive(Debug, Deserialize)]
pub struct SteamApiResponseInner {
    pub players: Vec<SteamPlayer>,
}

#[derive(Debug, Deserialize)]
pub struct SteamPlayer {
    #[serde(rename = "personaname")]
    pub persona_name: String,
    #[serde(rename = "avatarfull")]
    pub avatar_full: Option<String>,
}

/// Verify Steam OpenID identity by posting back to Steam's endpoint.
pub async fn verify_steam_identity(
    params: &SteamCallbackParams,
) -> Result<String, ApiError> {
    // Ensure mode is id_res (identity response from Steam)
    if params.openid_mode.as_deref() != Some("id_res") {
        tracing::warn!("invalid openid mode");
        return Err(ApiError::Unauthorized);
    }

    // Build verification request: same params but mode=check_authentication
    let mut verify_params = HashMap::new();
    verify_params.insert("openid.ns", params.openid_ns.as_deref().unwrap_or(""));
    verify_params.insert("openid.mode", "check_authentication");
    verify_params.insert(
        "openid.op_endpoint",
        params.openid_op_endpoint.as_deref().unwrap_or(""),
    );
    verify_params.insert(
        "openid.claimed_id",
        params.openid_claimed_id.as_deref().unwrap_or(""),
    );
    verify_params.insert("openid.identity", params.openid_identity.as_deref().unwrap_or(""));
    verify_params.insert("openid.return_to", params.openid_return_to.as_deref().unwrap_or(""));
    verify_params.insert(
        "openid.response_nonce",
        params.openid_response_nonce.as_deref().unwrap_or(""),
    );
    verify_params.insert(
        "openid.assoc_handle",
        params.openid_assoc_handle.as_deref().unwrap_or(""),
    );
    verify_params.insert("openid.signed", params.openid_signed.as_deref().unwrap_or(""));
    verify_params.insert("openid.sig", params.openid_sig.as_deref().unwrap_or(""));

    // POST to Steam's verification endpoint
    let client = reqwest::Client::new();
    let response = client
        .post("https://steamcommunity.com/openid/login")
        .form(&verify_params)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "steam verification request failed");
            ApiError::Internal(anyhow!("steam verification failed: {}", e))
        })?;

    let body = response
        .text()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to read steam response");
            ApiError::Internal(anyhow!("failed to read steam response: {}", e))
        })?;

    tracing::debug!(response_body = %body, "steam verification response received");

    // Check if is_valid:true in response
    if !body.contains("is_valid:true") {
        tracing::warn!("steam verification failed: is_valid not true");
        return Err(ApiError::Unauthorized);
    }

    // Extract Steam64 ID from openid.claimed_id URL
    // Format: https://steamcommunity.com/openid/id/{steam64id}
    let claimed_id = params
        .openid_claimed_id
        .as_deref()
        .ok_or_else(|| {
            tracing::warn!("no openid.claimed_id in steam response");
            ApiError::Unauthorized
        })?;

    let steam_id = claimed_id
        .strip_prefix("https://steamcommunity.com/openid/id/")
        .ok_or_else(|| {
            tracing::warn!(claimed_id = %claimed_id, "invalid claimed_id format");
            ApiError::Unauthorized
        })?
        .to_string();

    tracing::debug!(steam_id = %steam_id, "extracted steam_id from claimed_id");

    Ok(steam_id)
}

/// Create a new user from Steam player data.
pub async fn create_steam_user(
    state: &AppState,
    steam_id: &str,
) -> Result<user_repo::UserRow, ApiError> {
    // Try to fetch Steam player info if API key is configured
    let (username, avatar_url) = if let Some(api_key) = &state.config.steam_api_key {
        match fetch_steam_player_info(steam_id, api_key).await {
            Ok((name, avatar)) => (name, avatar),
            Err(e) => {
                tracing::warn!(error = %e, "failed to fetch steam player info, using fallback");
                (format!("steam_{}", steam_id), None)
            }
        }
    } else {
        tracing::debug!("no steam api key configured, using fallback username");
        (format!("steam_{}", steam_id), None)
    };

    // Generate random Ed25519-style placeholder public key (32 random bytes, hex-encoded)
    let random_bytes = rand::random::<[u8; 32]>();
    let public_key = hex::encode(random_bytes);

    // Generate Snowflake ID
    let mut generator = SnowflakeGenerator::new(0, 0);
    let user_id = generator.next_id();

    // Create user with steam_id, no email or password
    let user = user_repo::create_user(&state.db, user_id, &username, &public_key, None, None)
        .await
        .map_err(ApiError::Database)?;

    // Update steam_id
    user_repo::update_steam_id(&state.db, user_id, steam_id)
        .await
        .map_err(ApiError::Database)?;

    // Update avatar if available
    if let Some(avatar) = avatar_url {
        user_repo::update_avatar(&state.db, user_id, &avatar)
            .await
            .map_err(ApiError::Database)?;
    }

    tracing::info!(user_id = user.id, steam_id = %steam_id, username = %username, "steam user created");

    // Return updated user with steam_id set
    Ok(user_repo::UserRow {
        steam_id: Some(steam_id.to_string()),
        ..user
    })
}

/// Fetch player information from Steam Web API.
async fn fetch_steam_player_info(
    steam_id: &str,
    api_key: &str,
) -> Result<(String, Option<String>), ApiError> {
    let url = format!(
        "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v2/?key={}&steamids={}",
        api_key, steam_id
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "steam api request failed");
            ApiError::Internal(anyhow!("steam api request failed: {}", e))
        })?;

    let steam_api: SteamApiResponse = response
        .json()
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "failed to parse steam api response");
            ApiError::Internal(anyhow!("failed to parse steam api response: {}", e))
        })?;

    let player = steam_api
        .response
        .players
        .into_iter()
        .next()
        .ok_or_else(|| {
            tracing::warn!("no player data in steam api response");
            ApiError::Internal(anyhow!("no player data returned from steam api"))
        })?;

    let username = if player.persona_name.is_empty() {
        format!("steam_{}", steam_id)
    } else {
        player.persona_name
    };

    tracing::debug!(username = %username, "fetched steam player info");

    Ok((username, player.avatar_full))
}
