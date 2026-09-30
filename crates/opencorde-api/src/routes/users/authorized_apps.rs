//! GET/DELETE /api/v1/users/@me/authorized-apps — manage authorized OAuth apps.
//!
//! Lists all applications the user has authorized and allows revoking individual grants.
//! Data comes from the user_oauth_authorizations table joined with applications.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::AppState;
use crate::error::ApiError;
use crate::middleware::auth::AuthUser;
use opencorde_db::repos::user_oauth_authorization_repo;

/// Public-facing authorized app entry.
#[derive(Debug, Serialize)]
pub struct AuthorizedApp {
    pub id: String,
    pub application_id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub scope: String,
    pub authorized_at: DateTime<Utc>,
}

impl From<user_oauth_authorization_repo::AuthorizedAppRow> for AuthorizedApp {
    fn from(row: user_oauth_authorization_repo::AuthorizedAppRow) -> Self {
        AuthorizedApp {
            id: row.id.to_string(),
            application_id: row.application_id.to_string(),
            name: row.app_name,
            description: row.app_description,
            icon_url: row.app_icon_url,
            scope: row.scope,
            authorized_at: row.authorized_at,
        }
    }
}

/// GET /api/v1/users/@me/authorized-apps — list all authorized OAuth applications.
pub async fn list_authorized_apps(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<AuthorizedApp>>, ApiError> {
    let rows = user_oauth_authorization_repo::list_for_user(&state.db, auth.user_id.as_i64())
        .await
        .map_err(ApiError::Database)?;

    let apps: Vec<AuthorizedApp> = rows.into_iter().map(AuthorizedApp::from).collect();

    Ok(Json(apps))
}

/// DELETE /api/v1/users/@me/authorized-apps/{app_id} — revoke authorization for an app.
///
/// The {app_id} path parameter is the application's Snowflake ID.
pub async fn revoke_authorized_app(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(app_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let application_id: i64 = app_id
        .parse()
        .map_err(|_| ApiError::BadRequest("invalid application_id".into()))?;

    let deleted = user_oauth_authorization_repo::revoke_by_app(
        &state.db,
        auth.user_id.as_i64(),
        application_id,
    )
    .await
    .map_err(ApiError::Database)?;

    if !deleted {
        return Err(ApiError::NotFound("authorization not found".into()));
    }

    Ok(StatusCode::NO_CONTENT)
}
