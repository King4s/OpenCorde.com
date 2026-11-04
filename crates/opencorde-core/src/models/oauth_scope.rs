//! # Model: OAuthScope
//! OAuth2 authorization scope definitions.
//!
//! Scopes control what permissions an application requests when
//! a user authorizes it. These are seeded at migration time and
//! treated as a read-only enum in application code.

use serde::{Deserialize, Serialize};

/// OAuthScope: a named authorization scope.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OAuthScope {
    /// Numeric scope ID (matches database seed)
    pub id: i16,
    /// Scope name (e.g., "identify", "bot", "messages.read")
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
}

/// Well-known OAuth scope names for convenience.
pub mod scope_names {
    /// Read username, avatar, and public key
    pub const IDENTIFY: &str = "identify";
    /// Know what servers the user is in
    pub const GUILDS: &str = "guilds";
    /// Add a bot user to a server
    pub const BOT: &str = "bot";
    /// Read messages in channels the bot can see
    pub const MESSAGES_READ: &str = "messages.read";
    /// Create and manage slash commands
    pub const APPLICATIONS_COMMANDS: &str = "applications.commands";
    /// Create webhooks in the server
    pub const WEBHOOK_INCOMING: &str = "webhook.incoming";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_creation() {
        let scope = OAuthScope {
            id: 1,
            name: "identify".to_string(),
            description: Some("Read your profile info".to_string()),
        };

        assert_eq!(scope.id, 1);
        assert_eq!(scope.name, "identify");
        assert!(scope.description.is_some());
    }

    #[test]
    fn test_scope_names_constants() {
        assert_eq!(scope_names::IDENTIFY, "identify");
        assert_eq!(scope_names::BOT, "bot");
        assert_eq!(scope_names::MESSAGES_READ, "messages.read");
    }

    #[test]
    fn test_scope_serialization() {
        let scope = OAuthScope {
            id: 3,
            name: "bot".to_string(),
            description: Some("Add a bot to a server".to_string()),
        };

        let json = serde_json::to_string(&scope).unwrap();
        let deserialized: OAuthScope = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, scope.id);
        assert_eq!(deserialized.name, scope.name);
    }

    #[test]
    fn test_scope_equality() {
        let a = OAuthScope {
            id: 1,
            name: "identify".to_string(),
            description: None,
        };
        let b = OAuthScope {
            id: 1,
            name: "identify".to_string(),
            description: None,
        };
        let c = OAuthScope {
            id: 2,
            name: "guilds".to_string(),
            description: None,
        };

        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
