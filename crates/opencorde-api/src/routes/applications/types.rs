use serde::{Deserialize, Serialize};
use opencorde_core::models::application::Application;
use opencorde_core::models::bot_user::BotUser;

// === Request types ===

#[derive(Debug, Deserialize)]
pub struct CreateApplicationRequest {
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    #[serde(default)]
    pub is_public: bool,
    #[serde(default)]
    pub redirect_uris: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateApplicationRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub is_public: Option<bool>,
    pub redirect_uris: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBotRequest {
    pub username: String,
    pub avatar_url: Option<String>,
}

// === Response types ===

#[derive(Debug, Serialize)]
pub struct ApplicationResponse {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    pub owner_user_id: i64,
    pub flags: i64,
    pub is_public: bool,
    pub redirect_uris: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Application> for ApplicationResponse {
    fn from(a: Application) -> Self {
        Self {
            id: a.id.as_i64(),
            name: a.name,
            description: a.description,
            icon_url: a.icon_url,
            owner_user_id: a.owner_user_id.as_i64(),
            flags: a.flags,
            is_public: a.is_public,
            redirect_uris: a.redirect_uris,
            created_at: a.created_at.to_rfc3339(),
            updated_at: a.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BotUserResponse {
    pub id: i64,
    pub application_id: i64,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub created_at: String,
}

impl From<BotUser> for BotUserResponse {
    fn from(b: BotUser) -> Self {
        Self {
            id: b.id.as_i64(),
            application_id: b.application_id.as_i64(),
            username: b.username,
            avatar_url: b.avatar_url,
            created_at: b.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListMineResponse {
    pub applications: Vec<ApplicationResponse>,
}

#[derive(Debug, Serialize)]
pub struct ListPublicResponse {
    pub applications: Vec<ApplicationResponse>,
}
