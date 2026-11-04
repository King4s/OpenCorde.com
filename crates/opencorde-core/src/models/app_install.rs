//! # Model: AppInstall
//! Represents a bot/application installation to a server.
//!
//! Tracks which application is installed in which server,
//! what scopes were granted, and who performed the install.

use crate::snowflake::Snowflake;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// AppInstall: an application installation grant for a server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInstall {
    /// Unique install ID (Snowflake)
    pub id: Snowflake,
    /// Application being installed
    pub application_id: Snowflake,
    /// Server receiving the installation
    pub server_id: Snowflake,
    /// Admin who authorized the install
    pub installed_by: Snowflake,
    /// Granted OAuth scope names
    pub scopes: Vec<String>,
    /// Installation timestamp
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_install_creation() {
        let install = AppInstall {
            id: Snowflake::new(8000),
            application_id: Snowflake::new(1000),
            server_id: Snowflake::new(500),
            installed_by: Snowflake::new(200),
            scopes: vec!["bot".to_string(), "messages.read".to_string()],
            created_at: Utc::now(),
        };

        assert_eq!(install.application_id, Snowflake::new(1000));
        assert_eq!(install.server_id, Snowflake::new(500));
        assert_eq!(install.installed_by, Snowflake::new(200));
        assert_eq!(install.scopes.len(), 2);
    }

    #[test]
    fn test_app_install_serialization() {
        let install = AppInstall {
            id: Snowflake::new(9000),
            application_id: Snowflake::new(1000),
            server_id: Snowflake::new(500),
            installed_by: Snowflake::new(300),
            scopes: vec!["bot".to_string()],
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&install).unwrap();
        let deserialized: AppInstall = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, install.id);
        assert_eq!(deserialized.application_id, install.application_id);
        assert_eq!(deserialized.server_id, install.server_id);
        assert_eq!(deserialized.installed_by, install.installed_by);
        assert_eq!(deserialized.scopes, vec!["bot"]);
    }

    #[test]
    fn test_app_install_empty_scopes() {
        let install = AppInstall {
            id: Snowflake::new(10000),
            application_id: Snowflake::new(2000),
            server_id: Snowflake::new(600),
            installed_by: Snowflake::new(400),
            scopes: vec![],
            created_at: Utc::now(),
        };

        assert!(install.scopes.is_empty());
    }

    #[test]
    fn test_app_install_multiple_scopes() {
        let scopes = vec![
            "identify".to_string(),
            "guilds".to_string(),
            "bot".to_string(),
            "messages.read".to_string(),
        ];
        let install = AppInstall {
            id: Snowflake::new(11000),
            application_id: Snowflake::new(3000),
            server_id: Snowflake::new(700),
            installed_by: Snowflake::new(500),
            scopes: scopes.clone(),
            created_at: Utc::now(),
        };

        assert_eq!(install.scopes.len(), 4);
        assert!(install.scopes.contains(&"bot".to_string()));
    }
}
