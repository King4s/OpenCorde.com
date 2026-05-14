//! # Route: Slash Commands
//! Slash command registration and dispatch.
//!
//! ## Endpoints
//! - POST /api/v1/servers/{id}/commands — Register slash command (owner auth)
//! - GET /api/v1/servers/{id}/commands — List server commands (auth)
//! - DELETE /api/v1/commands/{command_id} — Delete command (owner auth)
//! - POST /api/v1/channels/{channel_id}/interact — Dispatch slash command interaction

use axum::{
    Router,
    routing::{delete, post},
};

use crate::AppState;

mod create;
mod delete;
mod dispatch;
mod list;
mod types;

// Re-export helpers from parent
use super::helpers;

pub use types::{CreateCommandRequest, InteractRequest, SlashCommandResponse};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/servers/{id}/commands",
            post(create::create_command).get(list::list_commands),
        )
        .route(
            "/api/v1/commands/{command_id}",
            delete(delete::delete_command),
        )
        .route(
            "/api/v1/channels/{channel_id}/interact",
            post(dispatch::dispatch_command),
        )
}
