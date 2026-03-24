//! Type definitions for slash commands.

use chrono::{DateTime, Utc};
use opencorde_db::repos::slash_command_repo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct SlashCommandResponse {
    pub id: String,
    pub server_id: String,
    pub name: String,
    pub description: String,
    pub handler_url: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommandRequest {
    pub name: String,
    pub description: Option<String>,
    pub handler_url: String,
}

#[derive(Debug, Deserialize)]
pub struct InteractRequest {
    pub command: String,
    pub args: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandHandlerPayload {
    pub command: String,
    pub args: Vec<String>,
    pub user_id: String,
    pub username: String,
    pub channel_id: String,
    pub server_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CommandHandlerResponse {
    pub content: String,
}

pub fn row_to_response(row: slash_command_repo::SlashCommandRow) -> SlashCommandResponse {
    SlashCommandResponse {
        id: row.id.to_string(),
        server_id: row.server_id.to_string(),
        name: row.name,
        description: row.description,
        handler_url: row.handler_url,
        created_by: row.created_by.to_string(),
        created_at: row.created_at,
    }
}
