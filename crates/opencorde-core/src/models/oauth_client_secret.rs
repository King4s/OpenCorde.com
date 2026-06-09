//! OAuth2 client secret — hashed credential for OAuth2 flows.

use crate::snowflake::Snowflake;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthClientSecret {
    pub id: Snowflake,
    pub application_id: Snowflake,
    pub secret_prefix: String,
    pub label: Option<String>,
    pub created_by: Snowflake,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Public-safe view — never includes secret hash or full secret
#[derive(Debug, Serialize)]
pub struct OAuthClientSecretPublic {
    pub id: i64,
    pub application_id: i64,
    pub secret_prefix: String,
    pub label: Option<String>,
    pub created_at: String,
    pub revoked: bool,
}
