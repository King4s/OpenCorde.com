//! # Route: GDPR Data Export
//! Allows users to download all their personal data.
//!
//! ## Endpoints
//! - GET /api/v1/users/@me/export — Download all personal data as JSON
//!
//! ## Depends On
//! - axum (web framework)
//! - sqlx (database queries)
//! - crate::AppState, crate::middleware::auth::AuthUser

use axum::{
    extract::State,
    http::{header, HeaderMap},
    response::IntoResponse,
    routing::get,
    Router,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};

/// Full data export response.
#[derive(Debug, Serialize)]
pub struct DataExport {
    pub exported_at: DateTime<Utc>,
    pub profile: UserExport,
    pub servers: Vec<ServerMembership>,
    pub messages_sent: Vec<MessageExport>,
    pub dm_channels: Vec<DmExport>,
    pub friendships: Vec<FriendExport>,
    pub files_uploaded: Vec<FileExport>,
}

#[derive(Debug, Serialize)]
pub struct UserExport {
    pub id: String,
    pub username: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ServerMembership {
    pub server_id: String,
    pub server_name: String,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MessageExport {
    pub id: String,
    pub channel_id: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DmExport {
    pub dm_id: String,
    pub other_username: String,
}

#[derive(Debug, Serialize)]
pub struct FriendExport {
    pub relationship_id: String,
    pub other_username: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct FileExport {
    pub id: String,
    pub filename: String,
    pub size_bytes: i64,
    pub content_type: String,
    pub uploaded_at: DateTime<Utc>,
}

/// GET /api/v1/users/@me/export — Download all personal data as JSON.
///
/// Returns a JSON file attachment with the user's complete personal data.
#[tracing::instrument(skip(state, auth))]
pub async fn export_user_data(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, ApiError> {
    let uid = auth.user_id.as_i64();
    tracing::info!(user_id = uid, "starting data export");

    // Profile
    let profile_row = sqlx::query("SELECT id, username, email, created_at FROM users WHERE id = $1")
        .bind(uid)
        .fetch_one(&state.db)
        .await
        .map_err(ApiError::Database)?;
    let profile = UserExport {
        id: profile_row.get::<i64, _>("id").to_string(),
        username: profile_row.get("username"),
        email: profile_row.get("email"),
        created_at: profile_row.get("created_at"),
    };

    // Server memberships
    let server_rows = sqlx::query(
        "SELECT sm.server_id, s.name, sm.joined_at FROM server_members sm \
         JOIN servers s ON s.id = sm.server_id WHERE sm.user_id = $1 ORDER BY sm.joined_at",
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    let servers: Vec<ServerMembership> = server_rows
        .iter()
        .map(|r| ServerMembership {
            server_id: r.get::<i64, _>("server_id").to_string(),
            server_name: r.get("name"),
            joined_at: r.get("joined_at"),
        })
        .collect();

    // All messages sent (no limit — GDPR requires complete data)
    let msg_rows = sqlx::query(
        "SELECT id, channel_id, content, created_at FROM messages \
         WHERE author_id = $1 ORDER BY created_at DESC",
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    let messages_sent: Vec<MessageExport> = msg_rows
        .iter()
        .map(|r| MessageExport {
            id: r.get::<i64, _>("id").to_string(),
            channel_id: r.get::<i64, _>("channel_id").to_string(),
            content: r.get("content"),
            created_at: r.get("created_at"),
        })
        .collect();

    // DM channels (via dm_channel_members join table)
    let dm_rows = sqlx::query(
        "SELECT dc.id, u.username FROM dm_channels dc \
         JOIN dm_channel_members m1 ON m1.dm_channel_id = dc.id AND m1.user_id = $1 \
         JOIN dm_channel_members m2 ON m2.dm_channel_id = dc.id AND m2.user_id <> $1 \
         JOIN users u ON u.id = m2.user_id \
         ORDER BY dc.created_at",
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    let dm_channels: Vec<DmExport> = dm_rows
        .iter()
        .map(|r| DmExport {
            dm_id: r.get::<i64, _>("id").to_string(),
            other_username: r.get("username"),
        })
        .collect();

    // Friendships (relationships table uses from_user/to_user columns)
    let friend_rows = sqlx::query(
        "SELECT r.id, u.username, r.status::text FROM relationships r \
         JOIN users u ON u.id = CASE WHEN r.from_user = $1 THEN r.to_user ELSE r.from_user END \
         WHERE r.from_user = $1 OR r.to_user = $1 ORDER BY r.created_at",
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    let friendships: Vec<FriendExport> = friend_rows
        .iter()
        .map(|r| FriendExport {
            relationship_id: r.get::<i64, _>("id").to_string(),
            other_username: r.get("username"),
            status: r.get("status"),
        })
        .collect();

    // Files uploaded
    let file_rows = sqlx::query(
        "SELECT id, filename, size, content_type, created_at FROM files \
         WHERE uploader_id = $1 ORDER BY created_at DESC",
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    let files_uploaded: Vec<FileExport> = file_rows
        .iter()
        .map(|r| FileExport {
            id: r.get::<i64, _>("id").to_string(),
            filename: r.get("filename"),
            size_bytes: r.get("size"),
            content_type: r.get("content_type"),
            uploaded_at: r.get("created_at"),
        })
        .collect();

    let export = DataExport {
        exported_at: Utc::now(),
        profile,
        servers,
        messages_sent,
        dm_channels,
        friendships,
        files_uploaded,
    };

    let json = serde_json::to_string_pretty(&export)
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    tracing::info!(user_id = uid, "data export completed");

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        "attachment; filename=\"opencorde-data-export.json\""
            .parse()
            .unwrap(),
    );

    Ok((headers, json))
}

pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/users/@me/export", get(export_user_data))
}
