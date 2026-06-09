//! Bot token — hashed gateway credential for bot authentication.

use crate::snowflake::Snowflake;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct BotToken {
    pub id: Snowflake,
    pub application_id: Snowflake,
    pub bot_user_id: Snowflake,
    pub token_prefix: String,
    pub token_hash: String,
    pub intents: i64,
    pub label: Option<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_by: Snowflake,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Public-safe view — never includes token hash or full token
#[derive(Debug, Serialize)]
pub struct BotTokenPublic {
    pub id: i64,
    pub bot_user_id: i64,
    pub token_prefix: String,
    pub intents: i64,
    pub label: Option<String>,
    pub last_used_at: Option<String>,
    pub created_at: String,
    pub revoked: bool,
}

/// Returned ONCE at creation
#[derive(Debug, Serialize)]
pub struct BotTokenCreated {
    pub id: i64,
    pub token: String,
    pub token_prefix: String,
    pub intents: i64,
    pub label: Option<String>,
    pub created_at: String,
    pub warning: &'static str,
}
