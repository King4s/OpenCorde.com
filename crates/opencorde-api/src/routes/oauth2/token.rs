//! OAuth2 token exchange — stub for future implementation.

use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub client_id: String,
    pub client_secret: String,
    pub code: String,
    pub grant_type: String,
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u32,
    pub refresh_token: String,
    pub scope: String,
}

pub async fn exchange_token(
    axum::extract::Form(req): axum::extract::Form<TokenRequest>,
) -> Json<TokenResponse> {
    // TODO: Full implementation — validate client, code, issue JWT
    Json(TokenResponse {
        access_token: "mock_token_placeholder".into(),
        token_type: "Bearer".into(),
        expires_in: 604800,
        refresh_token: "mock_refresh_placeholder".into(),
        scope: "identify".into(),
    })
}

pub async fn revoke_token(
    axum::extract::Form(req): axum::extract::Form<TokenRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "revoked": true }))
}
