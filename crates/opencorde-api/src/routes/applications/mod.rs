//! Route: Applications — OAuth2 application management.
//!
//! POST   /api/v1/applications        — create a new application
//! GET    /api/v1/applications/@me    — get applications owned by authenticated user
//! GET    /api/v1/applications/public — get public applications
//! GET    /api/v1/applications/{id}   — get an application by ID
//! PATCH  /api/v1/applications/{id}   — update an application (owner only)
//! DELETE /api/v1/applications/{id}   — delete an application (owner only)
//! POST   /api/v1/applications/{id}/bot — create a bot user (owner only)

use axum::{Router, routing::{get, post}};
use crate::AppState;

mod bot_create;
mod create;
mod delete;
mod get;
mod list;
mod types;
mod update;

pub use types::{
    ApplicationResponse, BotUserResponse,
    CreateApplicationRequest, CreateBotRequest,
    ListMineResponse, ListPublicResponse, UpdateApplicationRequest,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/applications", post(create::create_application))
        .route("/api/v1/applications/@me", get(list::list_my_applications))
        .route("/api/v1/applications/public", get(list::list_public_applications))
        .route("/api/v1/applications/{id}",
            get(get::get_application)
                .patch(update::update_application)
                .delete(delete::delete_application))
        .route("/api/v1/applications/{id}/bot", post(bot_create::create_bot_user))
}
