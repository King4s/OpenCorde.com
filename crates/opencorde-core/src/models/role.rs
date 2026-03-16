//! # Model: Role
//! Server role with permission set and styling.

use crate::permissions::Permissions;
use crate::snowflake::Snowflake;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Server role: permissions, color, and settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Unique role ID
    pub id: Snowflake,
    /// Server this role belongs to
    pub server_id: Snowflake,
    /// Role display name
    pub name: String,
    /// Permissions granted by this role
    pub permissions: Permissions,
    /// Optional RGB color (as u32)
    pub color: Option<u32>,
    /// Position in role hierarchy
    pub position: i32,
    /// Whether members can mention this role
    pub mentionable: bool,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_creation() {
        let role = Role {
            id: Snowflake::new(600),
            server_id: Snowflake::new(100),
            name: "Moderator".to_string(),
            permissions: Permissions::MANAGE_MESSAGES | Permissions::KICK_MEMBERS,
            color: Some(0xFF0000),
            position: 1,
            mentionable: true,
            created_at: Utc::now(),
        };

        assert_eq!(role.name, "Moderator");
        assert_eq!(role.color.unwrap(), 0xFF0000);
        assert!(role.mentionable);
    }

    #[test]
    fn test_role_with_no_color() {
        let role = Role {
            id: Snowflake::new(601),
            server_id: Snowflake::new(100),
            name: "Member".to_string(),
            permissions: Permissions::SEND_MESSAGES,
            color: None,
            position: 0,
            mentionable: false,
            created_at: Utc::now(),
        };

        assert!(role.color.is_none());
    }

    #[test]
    fn test_role_permissions() {
        let perms =
            Permissions::MANAGE_CHANNELS | Permissions::MANAGE_ROLES | Permissions::BAN_MEMBERS;
        let role = Role {
            id: Snowflake::new(602),
            server_id: Snowflake::new(100),
            name: "Admin".to_string(),
            permissions: perms,
            color: None,
            position: 10,
            mentionable: true,
            created_at: Utc::now(),
        };

        assert!(role.permissions.contains(Permissions::MANAGE_CHANNELS));
        assert!(role.permissions.contains(Permissions::BAN_MEMBERS));
        assert!(!role.permissions.contains(Permissions::SEND_MESSAGES));
    }

    #[test]
    fn test_role_serialization() {
        let role = Role {
            id: Snowflake::new(603),
            server_id: Snowflake::new(100),
            name: "TestRole".to_string(),
            permissions: Permissions::SEND_MESSAGES,
            color: Some(0x00FF00),
            position: 2,
            mentionable: true,
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&role).unwrap();
        let deserialized: Role = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, role.id);
        assert_eq!(deserialized.name, role.name);
        assert_eq!(deserialized.permissions, role.permissions);
    }

    #[test]
    fn test_role_position() {
        let role1 = Role {
            id: Snowflake::new(604),
            server_id: Snowflake::new(100),
            name: "High Role".to_string(),
            permissions: Permissions::empty(),
            color: None,
            position: 100,
            mentionable: false,
            created_at: Utc::now(),
        };

        let role2 = Role {
            id: Snowflake::new(605),
            server_id: Snowflake::new(100),
            name: "Low Role".to_string(),
            permissions: Permissions::empty(),
            color: None,
            position: 1,
            mentionable: false,
            created_at: Utc::now(),
        };

        assert!(role1.position > role2.position);
    }

    #[test]
    fn test_role_color_values() {
        let colors = vec![
            (0xFF0000, "red"),
            (0x00FF00, "green"),
            (0x0000FF, "blue"),
            (0xFFFFFF, "white"),
        ];

        for (color_val, _name) in colors {
            let role = Role {
                id: Snowflake::new(700),
                server_id: Snowflake::new(100),
                name: "Colored".to_string(),
                permissions: Permissions::empty(),
                color: Some(color_val),
                position: 0,
                mentionable: false,
                created_at: Utc::now(),
            };

            assert_eq!(role.color.unwrap(), color_val);
        }
    }
}
