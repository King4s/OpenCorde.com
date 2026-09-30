//! Route: OAuth2 — client secrets, authorize, token, revoke.
//!
//! POST   /api/v1/applications/{id}/oauth/secrets    — create client secret
//! GET    /api/v1/applications/{id}/oauth/secrets    — list secrets (prefix only)
//! DELETE /api/v1/applications/{id}/oauth/secrets/{sid} — revoke secret
//! GET    /api/v1/oauth2/authorize                   — authorization prompt
//! POST   /api/v1/oauth2/token                       — exchange code for token
//! POST   /api/v1/oauth2/revoke                      — revoke grant

use crate::AppState;
use axum::{
    Router,
    routing::{delete, get, post},
};

mod authorize;
mod clients;
mod token;

pub use clients::{CreateSecretResponse, ListSecretsResponse};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/applications/{id}/oauth/secrets",
            post(clients::create_secret).get(clients::list_secrets),
        )
        .route(
            "/api/v1/applications/{id}/oauth/secrets/{sid}",
            delete(clients::revoke_secret),
        )
        .route(
            "/api/v1/oauth2/authorize",
            get(authorize::authorize_page).post(authorize::approve),
        )
        .route("/api/v1/oauth2/token", post(token::exchange_token))
        .route("/api/v1/oauth2/revoke", post(token::revoke_token))
}
