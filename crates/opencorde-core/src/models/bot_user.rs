//! # Model: BotUser
//! Bot user account linked to an application.
//!
//! Bot users are special user accounts (is_bot=true) that represent
//! automated application identities in servers.

use crate::snowflake::Snowflake;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// BotUser: a bot identity linked to an application.
/// The `id` field is also a user ID — bots are users with is_bot=true.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotUser {
    /// Bot user ID (also a valid user ID)
    pub id: Snowflake,
    /// Owning application ID
    pub application_id: Snowflake,
    /// Bot display name
    pub username: String,
    /// Bot avatar URL
    pub avatar_url: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bot_user_creation() {
        let bot = BotUser {
            id: Snowflake::new(5000),
            application_id: Snowflake::new(1000),
            username: "HelperBot".to_string(),
            avatar_url: Some("https://example.com/bot.png".to_string()),
            created_at: Utc::now(),
        };

        assert_eq!(bot.username, "HelperBot");
        assert_eq!(bot.application_id, Snowflake::new(1000));
        assert!(bot.avatar_url.is_some());
    }

    #[test]
    fn test_bot_user_serialization() {
        let bot = BotUser {
            id: Snowflake::new(6000),
            application_id: Snowflake::new(1000),
            username: "TestBot".to_string(),
            avatar_url: None,
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&bot).unwrap();
        let deserialized: BotUser = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, bot.id);
        assert_eq!(deserialized.username, bot.username);
        assert_eq!(deserialized.application_id, bot.application_id);
        assert_eq!(deserialized.avatar_url, None);
    }

    #[test]
    fn test_bot_user_no_avatar() {
        let bot = BotUser {
            id: Snowflake::new(7000),
            application_id: Snowflake::new(2000),
            username: "NoAvatarBot".to_string(),
            avatar_url: None,
            created_at: Utc::now(),
        };

        assert_eq!(bot.username, "NoAvatarBot");
        assert!(bot.avatar_url.is_none());
    }
}
