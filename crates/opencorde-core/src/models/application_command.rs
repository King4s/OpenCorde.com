//! Application command — app-owned slash/user/message command.

use crate::snowflake::Snowflake;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationCommand {
    pub id: Snowflake,
    pub application_id: Snowflake,
    pub server_id: Option<Snowflake>,
    pub name: String,
    pub description: String,
    pub command_type: i16,
    pub options: serde_json::Value,
    pub default_member_permissions: Option<i64>,
    pub dm_permission: bool,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    pub id: Snowflake,
    pub application_id: Snowflake,
    pub token_hash: String,
    pub interaction_type: i16,
    pub command_id: Option<Snowflake>,
    pub server_id: Option<Snowflake>,
    pub channel_id: Option<Snowflake>,
    pub user_id: Snowflake,
    pub message_id: Option<Snowflake>,
    pub data: serde_json::Value,
    pub response_state: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub responded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ApplicationCommandResponse {
    pub id: i64,
    pub application_id: i64,
    pub server_id: Option<i64>,
    pub name: String,
    pub description: String,
    pub command_type: i16,
    pub options: serde_json::Value,
    pub version: i64,
}

#[derive(Debug, Serialize)]
pub struct InteractionResponse {
    pub id: i64,
    pub application_id: i64,
    pub interaction_type: i16,
    pub user_id: i64,
    pub response_state: String,
}
