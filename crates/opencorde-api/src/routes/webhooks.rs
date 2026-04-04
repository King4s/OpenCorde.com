//! # Route: Webhooks
//! Incoming webhook management and execution.
//!
//! ## Endpoints
//! - POST /api/v1/channels/{channel_id}/webhooks — Create webhook (authenticated)
//! - GET /api/v1/channels/{channel_id}/webhooks — List webhooks (authenticated)
//! - DELETE /api/v1/webhooks/{webhook_id} — Delete webhook (authenticated)
//! - POST /api/v1/webhooks/{token}/execute — Execute webhook (no auth, token-based)

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use opencorde_core::snowflake::SnowflakeGenerator;
use opencorde_db::repos::{message_repo, webhook_repo};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};
use crate::routes::{helpers::parse_snowflake, moderation::audit_mod::log_mod_action};

/// Public response for a webhook (excludes sensitive data).
#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub id: String,
    pub channel_id: String,
    pub server_id: String,
    pub name: String,
    pub token: String,
    pub url: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

/// Request body for creating a webhook.
#[derive(Debug, Deserialize)]
pub struct CreateWebhookRequest {
    pub name: Option<String>,
}

/// Request body for executing a webhook.
#[derive(Debug, Deserialize)]
pub struct ExecuteWebhookRequest {
    pub content: String,
    pub username: Option<String>,
}

/// Build the webhooks router with all endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/channels/{channel_id}/webhooks",
            post(create_webhook).get(list_webhooks),
        )
        .route("/api/v1/webhooks/{webhook_id}", delete(delete_webhook))
        .route("/api/v1/webhooks/{token}/execute", post(execute_webhook))
}

/// Convert WebhookRow to WebhookResponse.
fn webhook_row_to_response(row: webhook_repo::WebhookRow) -> WebhookResponse {
    WebhookResponse {
        id: row.id.to_string(),
        channel_id: row.channel_id.to_string(),
        server_id: row.server_id.to_string(),
        name: row.name,
        token: row.token.clone(),
        url: format!("/api/v1/webhooks/{}/execute", row.token),
        created_by: row.created_by.to_string(),
        created_at: row.created_at,
    }
}

/// POST /api/v1/channels/{channel_id}/webhooks — Create a new webhook.
///
/// Requires authentication. Returns 201 with the created webhook.
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn create_webhook(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
    Json(req): Json<CreateWebhookRequest>,
) -> Result<(StatusCode, Json<WebhookResponse>), ApiError> {
    let channel_id_sf = parse_snowflake(&channel_id)?;
    tracing::debug!(channel_id = channel_id_sf.as_i64(), "parsed channel id");

    // Verify channel exists and get server_id
    let channel_row: (i64,) = sqlx::query_as(
        "SELECT server_id FROM channels WHERE id = $1"
    )
    .bind(channel_id_sf.as_i64())
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("channel not found".into()))?;

    let server_id_sf = opencorde_core::snowflake::Snowflake::new(channel_row.0);

    // Generate token and ID
    let token = Uuid::new_v4().to_string().replace('-', "");
    let mut generator = SnowflakeGenerator::new(1, 1);
    let webhook_id = generator.next_id();
    let name = req.name.unwrap_or_else(|| "Webhook".to_string());

    tracing::info!(
        webhook_id = webhook_id.as_i64(),
        channel_id = channel_id_sf.as_i64(),
        name = %name,
        "creating webhook"
    );

    let webhook = webhook_repo::create_webhook(
        &state.db,
        webhook_id,
        channel_id_sf,
        server_id_sf,
        &name,
        &token,
        auth.user_id,
    )
    .await
    .map_err(ApiError::Database)?;

    let wh_id = webhook.id;
    log_mod_action(&state, server_id_sf, auth.user_id, "webhook.create", wh_id).await;
    Ok((StatusCode::CREATED, Json(webhook_row_to_response(webhook))))
}

/// GET /api/v1/channels/{channel_id}/webhooks — List webhooks for a channel.
///
/// Requires authentication. Returns 200 with webhook list.
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn list_webhooks(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
) -> Result<Json<Vec<WebhookResponse>>, ApiError> {
    let channel_id_sf = parse_snowflake(&channel_id)?;
    tracing::debug!(channel_id = channel_id_sf.as_i64(), "listing webhooks");

    let webhooks = webhook_repo::list_by_channel(&state.db, channel_id_sf)
        .await
        .map_err(ApiError::Database)?;

    let responses = webhooks.into_iter().map(webhook_row_to_response).collect();
    Ok(Json(responses))
}

/// DELETE /api/v1/webhooks/{webhook_id} — Delete a webhook.
///
/// Requires authentication. Verifies ownership before deletion.
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn delete_webhook(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(webhook_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let webhook_id_sf = parse_snowflake(&webhook_id)?;
    tracing::debug!(webhook_id = webhook_id_sf.as_i64(), "deleting webhook");

    let webhook = webhook_repo::get_by_id(&state.db, webhook_id_sf)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("webhook not found".into()))?;

    // Verify ownership
    if webhook.created_by != auth.user_id.as_i64() {
        tracing::warn!(
            webhook_id = webhook_id_sf.as_i64(),
            user_id = auth.user_id.as_i64(),
            "user does not own webhook"
        );
        return Err(ApiError::Forbidden);
    }

    webhook_repo::delete_webhook(&state.db, webhook_id_sf)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(webhook_id = webhook_id_sf.as_i64(), "webhook deleted");
    let server_id_sf = opencorde_core::Snowflake::new(webhook.server_id);
    log_mod_action(&state, server_id_sf, auth.user_id, "webhook.delete", webhook_id_sf.as_i64()).await;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/webhooks/{token}/execute — Execute a webhook.
///
/// No authentication required — token acts as authorization.
/// Creates a message in the webhook's channel.
#[tracing::instrument(skip(state, req))]
async fn execute_webhook(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(req): Json<ExecuteWebhookRequest>,
) -> Result<StatusCode, ApiError> {
    tracing::debug!("executing webhook");

    // Look up webhook by token
    let webhook = webhook_repo::get_by_token(&state.db, &token)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("webhook token not found".into()))?;

    let channel_id_sf = opencorde_core::snowflake::Snowflake::new(webhook.channel_id);
    let creator_user_id = opencorde_core::snowflake::Snowflake::new(webhook.created_by);

    // Validate content length
    if req.content.is_empty() || req.content.len() > 2000 {
        return Err(ApiError::BadRequest(
            "content must be 1-2000 characters".into(),
        ));
    }

    // Prepend username if provided
    let content = if let Some(username) = req.username {
        format!("**{}**: {}", username, req.content)
    } else {
        req.content
    };

    // Generate message ID
    let mut generator = SnowflakeGenerator::new(1, 1);
    let msg_id = generator.next_id();

    tracing::info!(
        webhook_id = webhook.id,
        channel_id = webhook.channel_id,
        "creating webhook message"
    );

    // Create message in the channel
    message_repo::create_message(
        &state.db,
        msg_id,
        channel_id_sf,
        creator_user_id,
        &content,
        None,
        serde_json::json!([]),
        None,
    )
    .await
    .map_err(ApiError::Database)?;

    tracing::info!(msg_id = msg_id.as_i64(), "webhook message created");
    Ok(StatusCode::NO_CONTENT)
}
