//! # Routes Module
//! HTTP route handlers for the REST API.
//!
//! ## Modules
//! - `health` — Liveness/readiness probe endpoint
//! - `auth` — Authentication (register, login, refresh)
//! - `users` — User profile management
//! - `servers` — Server CRUD operations
//! - `channels` — Channel CRUD operations
//! - `channel_overrides` — Per-channel permission overrides (role/member)
//! - `messages` — Message CRUD and typing indicator
//! - `reactions` — Emoji reactions to messages
//! - `read_state` — Unread tracking and mark-as-read
//! - `moderation` — Ban, kick, timeout management
//! - `dms` — Direct message channels
//! - `e2ee` — End-to-end encryption key exchange (key packages, MLS groups)
//! - `push` — Push notification token registration/unregistration
//!
//! ## Depends On
//! - axum (web framework)
//! - crate::AppState (application state)

pub mod admin;
pub mod audit_log;
pub mod data_export;
pub mod auth;
pub mod e2ee;
pub mod automod;
pub mod channel_overrides;
pub mod channels;
pub mod discovery;
pub mod dms;
pub mod emojis;
pub mod events;
pub mod forum;
pub mod friends;
pub mod health;
pub mod helpers;
pub mod invites;
pub mod members;
pub mod federation;
pub mod mesh;
pub mod messages;
pub mod moderation;
pub mod permission_check;
pub mod pins;
pub mod reactions;
pub mod read_state;
pub mod recordings;
pub mod roles;
pub mod search;
pub mod servers;
pub mod slash_commands;
pub mod stage;
pub mod threads;
pub mod notification_settings;
pub mod unfurl;
pub mod upload_validation;
pub mod uploads;
pub mod users;
pub mod push;
pub mod voice;
pub mod webhooks;

use axum::Router;

use crate::AppState;

/// Build the complete API router with all routes.
pub fn api_router() -> Router<AppState> {
    Router::new()
        .merge(admin::router())
        .merge(health::router())
        .merge(auth::router())
        .merge(users::router())
        .merge(servers::router())
        .merge(channels::router())
        .merge(channel_overrides::router())
        .merge(discovery::router())
        .merge(events::router())
        .merge(emojis::router())
        .merge(forum::router())
        .merge(invites::router())
        .merge(members::router())
        .merge(mesh::router())
        .merge(federation::router())
        .merge(roles::router())
        .merge(messages::router())
        .merge(threads::router())
        .merge(pins::router())
        .merge(reactions::router())
        .merge(read_state::router())
        .merge(moderation::router())
        .merge(automod::router())
        .merge(slash_commands::router())
        .merge(stage::router())
        .merge(dms::router())
        .merge(friends::router())
        .merge(search::router())
        .merge(uploads::router())
        .merge(voice::router())
        .merge(recordings::router())
        .merge(webhooks::router())
        .merge(audit_log::router())
        .merge(data_export::router())
        .merge(e2ee::router())
        .merge(notification_settings::router())
        .merge(unfurl::router())
        .merge(push::router())
        .merge(crate::ws::handler::router())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_router_creation() {
        let _router = api_router();
        // Verify all route modules can be composed without panic
    }
}
