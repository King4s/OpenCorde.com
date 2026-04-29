//! # Route: Effective Permissions
//! Inspect resolved server and channel permissions for a user.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use opencorde_core::{Snowflake, permissions::Permissions};
use serde::Serialize;

use crate::{
    AppState, error::ApiError, middleware::auth::AuthUser, routes::helpers,
    routes::permission_check,
};

#[derive(Debug, Serialize)]
pub struct EffectivePermissionsResponse {
    pub user_id: String,
    pub server_id: String,
    pub channel_id: Option<String>,
    pub permissions: u64,
    pub names: Vec<&'static str>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/servers/{server_id}/permissions/{user_id}/effective",
            get(get_server_effective_permissions),
        )
        .route(
            "/api/v1/channels/{channel_id}/permissions/{user_id}/effective",
            get(get_channel_effective_permissions),
        )
}

async fn get_server_effective_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, user_id)): Path<(String, String)>,
) -> Result<Json<EffectivePermissionsResponse>, ApiError> {
    let server_id = helpers::parse_snowflake(&server_id)?;
    let target_id = helpers::parse_snowflake(&user_id)?;

    permission_check::require_server_perm(
        &state.db,
        auth.user_id,
        server_id,
        Permissions::MANAGE_ROLES,
    )
    .await?;

    let permissions =
        permission_check::effective_server_perms(&state.db, target_id, server_id).await?;

    Ok(Json(response(target_id, server_id, None, permissions)))
}

async fn get_channel_effective_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((channel_id, user_id)): Path<(String, String)>,
) -> Result<Json<EffectivePermissionsResponse>, ApiError> {
    let channel_id = helpers::parse_snowflake(&channel_id)?;
    let target_id = helpers::parse_snowflake(&user_id)?;
    let server_id = permission_check::resolve_channel_server(&state.db, channel_id).await?;

    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id,
        Permissions::MANAGE_ROLES,
    )
    .await?;

    let permissions =
        permission_check::effective_channel_perms(&state.db, target_id, channel_id).await?;

    Ok(Json(response(
        target_id,
        server_id,
        Some(channel_id),
        permissions,
    )))
}

fn response(
    user_id: Snowflake,
    server_id: Snowflake,
    channel_id: Option<Snowflake>,
    permissions: Permissions,
) -> EffectivePermissionsResponse {
    EffectivePermissionsResponse {
        user_id: user_id.as_i64().to_string(),
        server_id: server_id.as_i64().to_string(),
        channel_id: channel_id.map(|id| id.as_i64().to_string()),
        permissions: permissions.bits(),
        names: permission_names(permissions),
    }
}

fn permission_names(permissions: Permissions) -> Vec<&'static str> {
    permission_definitions()
        .iter()
        .filter_map(|(name, flag)| permissions.contains(*flag).then_some(*name))
        .collect()
}

fn permission_definitions() -> &'static [(&'static str, Permissions)] {
    &[
        ("CREATE_INVITE", Permissions::CREATE_INVITE),
        ("KICK_MEMBERS", Permissions::KICK_MEMBERS),
        ("BAN_MEMBERS", Permissions::BAN_MEMBERS),
        ("ADMINISTRATOR", Permissions::ADMINISTRATOR),
        ("MANAGE_CHANNELS", Permissions::MANAGE_CHANNELS),
        ("MANAGE_SERVER", Permissions::MANAGE_SERVER),
        ("ADD_REACTIONS", Permissions::ADD_REACTIONS),
        ("VIEW_AUDIT_LOG", Permissions::VIEW_AUDIT_LOG),
        ("PRIORITY_SPEAKER", Permissions::PRIORITY_SPEAKER),
        ("STREAM", Permissions::STREAM),
        ("VIEW_CHANNEL", Permissions::VIEW_CHANNEL),
        ("SEND_MESSAGES", Permissions::SEND_MESSAGES),
        ("SEND_TTS_MESSAGES", Permissions::SEND_TTS_MESSAGES),
        ("MANAGE_MESSAGES", Permissions::MANAGE_MESSAGES),
        ("EMBED_LINKS", Permissions::EMBED_LINKS),
        ("ATTACH_FILES", Permissions::ATTACH_FILES),
        ("READ_MESSAGE_HISTORY", Permissions::READ_MESSAGE_HISTORY),
        ("MENTION_EVERYONE", Permissions::MENTION_EVERYONE),
        ("USE_EXTERNAL_EMOJIS", Permissions::USE_EXTERNAL_EMOJIS),
        ("VIEW_GUILD_INSIGHTS", Permissions::VIEW_GUILD_INSIGHTS),
        ("CONNECT", Permissions::CONNECT),
        ("SPEAK", Permissions::SPEAK),
        ("MUTE_MEMBERS", Permissions::MUTE_MEMBERS),
        ("DEAFEN_MEMBERS", Permissions::DEAFEN_MEMBERS),
        ("MOVE_MEMBERS", Permissions::MOVE_MEMBERS),
        ("USE_VAD", Permissions::USE_VAD),
        ("CHANGE_NICKNAME", Permissions::CHANGE_NICKNAME),
        ("MANAGE_NICKNAMES", Permissions::MANAGE_NICKNAMES),
        ("MANAGE_ROLES", Permissions::MANAGE_ROLES),
        ("MANAGE_WEBHOOKS", Permissions::MANAGE_WEBHOOKS),
        (
            "MANAGE_GUILD_EXPRESSIONS",
            Permissions::MANAGE_GUILD_EXPRESSIONS,
        ),
        (
            "USE_APPLICATION_COMMANDS",
            Permissions::USE_APPLICATION_COMMANDS,
        ),
        ("REQUEST_TO_SPEAK", Permissions::REQUEST_TO_SPEAK),
        ("MANAGE_EVENTS", Permissions::MANAGE_EVENTS),
        ("MANAGE_THREADS", Permissions::MANAGE_THREADS),
        ("CREATE_PUBLIC_THREADS", Permissions::CREATE_PUBLIC_THREADS),
        (
            "CREATE_PRIVATE_THREADS",
            Permissions::CREATE_PRIVATE_THREADS,
        ),
        ("USE_EXTERNAL_STICKERS", Permissions::USE_EXTERNAL_STICKERS),
        (
            "SEND_MESSAGES_IN_THREADS",
            Permissions::SEND_MESSAGES_IN_THREADS,
        ),
        ("MODERATE_MEMBERS", Permissions::MODERATE_MEMBERS),
        ("CREATE_EVENTS", Permissions::CREATE_EVENTS),
        ("SEND_VOICE_MESSAGES", Permissions::SEND_VOICE_MESSAGES),
        ("SEND_POLLS", Permissions::SEND_POLLS),
        ("PIN_MESSAGES", Permissions::PIN_MESSAGES),
        ("BYPASS_SLOWMODE", Permissions::BYPASS_SLOWMODE),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_include_known_flags() {
        let names = permission_names(Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES);
        assert_eq!(names, vec!["VIEW_CHANNEL", "SEND_MESSAGES"]);
    }
}
