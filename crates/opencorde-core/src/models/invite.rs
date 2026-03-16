//! # Model: Invite
//! Server invite links with expiration and usage limits.

use crate::snowflake::Snowflake;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Server invite: shareable link with usage limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    /// Unique invite code
    pub code: String,
    /// Server ID this invite is for
    pub server_id: Snowflake,
    /// User who created the invite
    pub creator_id: Snowflake,
    /// Number of times the invite has been used
    pub uses: i32,
    /// Maximum uses (None = unlimited)
    pub max_uses: Option<i32>,
    /// Expiration time (None = never expires)
    pub expires_at: Option<DateTime<Utc>>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl Invite {
    /// Checks if the invite has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            Utc::now() > expires
        } else {
            false
        }
    }

    /// Checks if the invite has reached max uses.
    pub fn is_exhausted(&self) -> bool {
        if let Some(max) = self.max_uses {
            self.uses >= max
        } else {
            false
        }
    }

    /// Checks if the invite is currently valid (not expired and not exhausted).
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.is_exhausted()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invite_creation() {
        let invite = Invite {
            code: "ABC123DEF".to_string(),
            server_id: Snowflake::new(100),
            creator_id: Snowflake::new(200),
            uses: 0,
            max_uses: Some(10),
            expires_at: None,
            created_at: Utc::now(),
        };

        assert_eq!(invite.code, "ABC123DEF");
        assert_eq!(invite.uses, 0);
    }

    #[test]
    fn test_invite_unlimited() {
        let invite = Invite {
            code: "UNLIMITED".to_string(),
            server_id: Snowflake::new(100),
            creator_id: Snowflake::new(200),
            uses: 5,
            max_uses: None,
            expires_at: None,
            created_at: Utc::now(),
        };

        assert!(!invite.is_exhausted());
        assert!(!invite.is_expired());
        assert!(invite.is_valid());
    }

    #[test]
    fn test_invite_exhausted() {
        let invite = Invite {
            code: "LIMITED".to_string(),
            server_id: Snowflake::new(100),
            creator_id: Snowflake::new(200),
            uses: 10,
            max_uses: Some(10),
            expires_at: None,
            created_at: Utc::now(),
        };

        assert!(invite.is_exhausted());
        assert!(!invite.is_valid());
    }

    #[test]
    fn test_invite_expired() {
        let now = Utc::now();
        let past = now - chrono::Duration::hours(1);

        let invite = Invite {
            code: "EXPIRED".to_string(),
            server_id: Snowflake::new(100),
            creator_id: Snowflake::new(200),
            uses: 2,
            max_uses: Some(10),
            expires_at: Some(past),
            created_at: now,
        };

        assert!(invite.is_expired());
        assert!(!invite.is_valid());
    }

    #[test]
    fn test_invite_valid() {
        let now = Utc::now();
        let future = now + chrono::Duration::days(7);

        let invite = Invite {
            code: "VALID".to_string(),
            server_id: Snowflake::new(100),
            creator_id: Snowflake::new(200),
            uses: 0,
            max_uses: Some(100),
            expires_at: Some(future),
            created_at: now,
        };

        assert!(!invite.is_expired());
        assert!(!invite.is_exhausted());
        assert!(invite.is_valid());
    }

    #[test]
    fn test_invite_serialization() {
        let invite = Invite {
            code: "SERIALIZE".to_string(),
            server_id: Snowflake::new(100),
            creator_id: Snowflake::new(200),
            uses: 3,
            max_uses: Some(50),
            expires_at: None,
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&invite).unwrap();
        let deserialized: Invite = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.code, invite.code);
        assert_eq!(deserialized.uses, invite.uses);
        assert_eq!(deserialized.max_uses, invite.max_uses);
    }

    #[test]
    fn test_invite_near_expiration() {
        let now = Utc::now();
        let soon = now + chrono::Duration::minutes(5);

        let invite = Invite {
            code: "SOON".to_string(),
            server_id: Snowflake::new(100),
            creator_id: Snowflake::new(200),
            uses: 0,
            max_uses: None,
            expires_at: Some(soon),
            created_at: now,
        };

        // Should still be valid but expiring soon
        assert!(invite.is_valid());
        assert!(!invite.is_expired());
    }

    #[test]
    fn test_invite_nearly_exhausted() {
        let invite = Invite {
            code: "NEARLY".to_string(),
            server_id: Snowflake::new(100),
            creator_id: Snowflake::new(200),
            uses: 9,
            max_uses: Some(10),
            expires_at: None,
            created_at: Utc::now(),
        };

        assert!(!invite.is_exhausted());
        assert!(invite.is_valid());
    }
}
