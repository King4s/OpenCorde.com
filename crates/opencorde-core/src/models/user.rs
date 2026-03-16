//! # Model: User
//! User account data including credentials and profile info.
//!
//! ## Identity Model
//! Users have a primary Ed25519 keypair identity (public_key) that is non-custodial
//! and portable across servers in the mesh. Email+password is optional for recovery.

use crate::snowflake::Snowflake;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// User account: full internal representation (includes password hash).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique user ID (Snowflake timestamp-ordered)
    pub id: Snowflake,
    /// Display name
    pub username: String,
    /// Ed25519 public key (hex-encoded, 64 chars). Primary identity across mesh.
    pub public_key: String,
    /// Email address (optional: only used for password recovery)
    pub email: Option<String>,
    /// Argon2 password hash (optional: only if email is set)
    pub password_hash: Option<String>,
    /// Optional avatar URL
    pub avatar_url: Option<String>,
    /// Current online status
    pub status: UserStatus,
    /// Account creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last profile update
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Converts to a public profile (removes email and password_hash).
    pub fn to_profile(&self) -> UserProfile {
        UserProfile {
            id: self.id,
            username: self.username.clone(),
            public_key: self.public_key.clone(),
            avatar_url: self.avatar_url.clone(),
            status: self.status,
        }
    }
}

/// User status: represents online/away state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    /// User is actively using the platform
    Online,
    /// User is away but active
    Idle,
    /// User does not want to be disturbed
    DoNotDisturb,
    /// User is offline
    Offline,
}

/// Public user profile: safe to expose in APIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// Unique user ID
    pub id: Snowflake,
    /// Display name
    pub username: String,
    /// Ed25519 public key (hex-encoded, 64 chars). User's mesh identity.
    pub public_key: String,
    /// Optional avatar URL
    pub avatar_url: Option<String>,
    /// Current status
    pub status: UserStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User {
            id: Snowflake::new(123),
            username: "alice".to_string(),
            public_key: "abc123def456abc123def456abc123def456abc123def456abc123def456abc1"
                .to_string(),
            email: Some("alice@example.com".to_string()),
            password_hash: Some("$2b$12$...hash...".to_string()),
            avatar_url: None,
            status: UserStatus::Online,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(user.username, "alice");
        assert_eq!(user.status, UserStatus::Online);
        assert_eq!(user.public_key.len(), 64);
    }

    #[test]
    fn test_user_to_profile() {
        let user = User {
            id: Snowflake::new(456),
            username: "bob".to_string(),
            public_key: "def789abc456def789abc456def789abc456def789abc456def789abc456def7"
                .to_string(),
            email: Some("bob@example.com".to_string()),
            password_hash: Some("$2b$12$...hash...".to_string()),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
            status: UserStatus::Idle,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let profile = user.to_profile();
        assert_eq!(profile.id, user.id);
        assert_eq!(profile.username, user.username);
        assert_eq!(profile.public_key, user.public_key);
        assert_eq!(profile.avatar_url, user.avatar_url);
        assert_eq!(profile.status, user.status);
    }

    #[test]
    fn test_user_status_serialization() {
        let status = UserStatus::Online;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"online\"");

        let deserialized: UserStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, status);
    }

    #[test]
    fn test_user_serialization() {
        let user = User {
            id: Snowflake::new(789),
            username: "charlie".to_string(),
            public_key: "ghi012jkl345ghi012jkl345ghi012jkl345ghi012jkl345ghi012jkl345ghi0"
                .to_string(),
            email: Some("charlie@example.com".to_string()),
            password_hash: Some("hash".to_string()),
            avatar_url: None,
            status: UserStatus::Offline,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, user.id);
        assert_eq!(deserialized.username, user.username);
        assert_eq!(deserialized.public_key, user.public_key);
    }

    #[test]
    fn test_user_profile_serialization() {
        let profile = UserProfile {
            id: Snowflake::new(111),
            username: "david".to_string(),
            public_key: "mno345pqr678mno345pqr678mno345pqr678mno345pqr678mno345pqr678mno3"
                .to_string(),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
            status: UserStatus::Online,
        };

        let json = serde_json::to_string(&profile).unwrap();
        let deserialized: UserProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, profile.id);
        assert_eq!(deserialized.username, profile.username);
        assert_eq!(deserialized.public_key, profile.public_key);
    }
}
