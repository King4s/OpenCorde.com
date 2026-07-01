//! # Model: UserOAuthAuthorization
//! Records a user's authorization grant to an OAuth2 application.
//!
//! Created when a user approves an OAuth2 authorization request.
//! Referenced by the authorized-apps page to list and revoke grants.

use crate::snowflake::Snowflake;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// UserOAuthAuthorization: a user-to-application authorization grant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOAuthAuthorization {
    /// Unique authorization ID (Snowflake)
    pub id: Snowflake,
    /// User who authorized the application
    pub user_id: Snowflake,
    /// Application the user authorized
    pub application_id: Snowflake,
    /// Granted OAuth scope string
    pub scope: String,
    /// When the authorization was granted
    pub authorized_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization_creation() {
        let auth = UserOAuthAuthorization {
            id: Snowflake::new(1000),
            user_id: Snowflake::new(200),
            application_id: Snowflake::new(300),
            scope: "identify".to_string(),
            authorized_at: Utc::now(),
        };

        assert_eq!(auth.user_id, Snowflake::new(200));
        assert_eq!(auth.application_id, Snowflake::new(300));
        assert_eq!(auth.scope, "identify");
    }

    #[test]
    fn test_authorization_serialization() {
        let auth = UserOAuthAuthorization {
            id: Snowflake::new(2000),
            user_id: Snowflake::new(400),
            application_id: Snowflake::new(500),
            scope: "identify guilds".to_string(),
            authorized_at: Utc::now(),
        };

        let json = serde_json::to_string(&auth).unwrap();
        let deserialized: UserOAuthAuthorization = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, auth.id);
        assert_eq!(deserialized.scope, "identify guilds");
    }
}
