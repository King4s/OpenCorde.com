//! # Model: Member
//! Server membership: user roles and join info.

use crate::snowflake::Snowflake;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Server member: a user with roles in a server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    /// The user's ID
    pub user_id: Snowflake,
    /// The server's ID
    pub server_id: Snowflake,
    /// Optional custom nickname in this server
    pub nickname: Option<String>,
    /// IDs of roles assigned to this member
    pub role_ids: Vec<Snowflake>,
    /// When the user joined the server
    pub joined_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_member_creation() {
        let member = Member {
            user_id: Snowflake::new(200),
            server_id: Snowflake::new(100),
            nickname: None,
            role_ids: vec![],
            joined_at: Utc::now(),
        };

        assert_eq!(member.user_id, Snowflake::new(200));
        assert_eq!(member.server_id, Snowflake::new(100));
        assert!(member.role_ids.is_empty());
    }

    #[test]
    fn test_member_with_nickname() {
        let member = Member {
            user_id: Snowflake::new(201),
            server_id: Snowflake::new(100),
            nickname: Some("Cool Guy".to_string()),
            role_ids: vec![],
            joined_at: Utc::now(),
        };

        assert_eq!(member.nickname.as_ref().unwrap(), "Cool Guy");
    }

    #[test]
    fn test_member_with_roles() {
        let member = Member {
            user_id: Snowflake::new(202),
            server_id: Snowflake::new(100),
            nickname: None,
            role_ids: vec![
                Snowflake::new(300),
                Snowflake::new(301),
                Snowflake::new(302),
            ],
            joined_at: Utc::now(),
        };

        assert_eq!(member.role_ids.len(), 3);
        assert!(member.role_ids.contains(&Snowflake::new(300)));
    }

    #[test]
    fn test_member_serialization() {
        let member = Member {
            user_id: Snowflake::new(203),
            server_id: Snowflake::new(100),
            nickname: Some("Test User".to_string()),
            role_ids: vec![Snowflake::new(400), Snowflake::new(401)],
            joined_at: Utc::now(),
        };

        let json = serde_json::to_string(&member).unwrap();
        let deserialized: Member = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.user_id, member.user_id);
        assert_eq!(deserialized.nickname, member.nickname);
        assert_eq!(deserialized.role_ids.len(), 2);
    }

    #[test]
    fn test_member_without_nickname() {
        let member = Member {
            user_id: Snowflake::new(204),
            server_id: Snowflake::new(100),
            nickname: None,
            role_ids: vec![Snowflake::new(500)],
            joined_at: Utc::now(),
        };

        assert!(member.nickname.is_none());
    }
}
