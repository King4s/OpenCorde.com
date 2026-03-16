//! # Permission Bitfield
//! 64-bit permission system with role-based overrides.
//!
//! Uses bitflags for efficient storage and composition.
//! Supports per-channel permission overwrites for roles and members.

use crate::snowflake::Snowflake;
use bitflags::bitflags;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

bitflags! {
    /// Permission flags for roles and channels.
    /// Follows Discord-like permission model.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Permissions: u64 {
        // General
        /// View channels and basic information
        const VIEW_CHANNEL       = 1 << 0;
        /// Create, edit, and delete channels
        const MANAGE_CHANNELS    = 1 << 1;
        /// Create and manage roles
        const MANAGE_ROLES       = 1 << 2;
        /// Full server administration
        const MANAGE_SERVER      = 1 << 3;
        /// Create invites to the server
        const CREATE_INVITE      = 1 << 4;
        /// Change own nickname
        const CHANGE_NICKNAME    = 1 << 5;
        /// Change others' nicknames
        const MANAGE_NICKNAMES   = 1 << 6;
        /// Remove members from server
        const KICK_MEMBERS       = 1 << 7;
        /// Permanently remove members
        const BAN_MEMBERS        = 1 << 8;

        // Text
        /// Send messages in text channels
        const SEND_MESSAGES      = 1 << 10;
        /// Embed links in messages
        const EMBED_LINKS        = 1 << 11;
        /// Attach files to messages
        const ATTACH_FILES       = 1 << 12;
        /// Read message history
        const READ_MESSAGE_HISTORY = 1 << 13;
        /// Mention @everyone
        const MENTION_EVERYONE   = 1 << 14;
        /// Edit/delete others' messages
        const MANAGE_MESSAGES    = 1 << 15;
        /// Add reactions to messages
        const ADD_REACTIONS      = 1 << 16;

        // Voice
        /// Connect to voice channels
        const CONNECT            = 1 << 20;
        /// Speak in voice channels
        const SPEAK              = 1 << 21;
        /// Stream video in voice channels
        const VIDEO              = 1 << 22;
        /// Mute other members in voice
        const MUTE_MEMBERS       = 1 << 23;
        /// Deafen other members in voice
        const DEAFEN_MEMBERS     = 1 << 24;
        /// Move members between voice channels
        const MOVE_MEMBERS       = 1 << 25;
        /// Speak with priority in voice (stage)
        const PRIORITY_SPEAKER   = 1 << 26;

        // Admin
        /// Bypass all permission checks
        const ADMINISTRATOR      = 1 << 30;
    }
}

impl Permissions {
    /// Default permissions granted to @everyone role.
    /// Allows basic interaction: view, send messages, read history, voice, invite.
    pub fn default_everyone() -> Self {
        Permissions::VIEW_CHANNEL
            | Permissions::SEND_MESSAGES
            | Permissions::READ_MESSAGE_HISTORY
            | Permissions::CONNECT
            | Permissions::SPEAK
            | Permissions::ADD_REACTIONS
            | Permissions::EMBED_LINKS
            | Permissions::ATTACH_FILES
            | Permissions::CREATE_INVITE
            | Permissions::CHANGE_NICKNAME
    }

    /// All permissions set.
    pub fn all_permissions() -> Self {
        Permissions::all()
    }
}

impl Serialize for Permissions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.bits())
    }
}

impl<'de> Deserialize<'de> for Permissions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bits = u64::deserialize(deserializer)?;
        Ok(Permissions::from_bits_truncate(bits))
    }
}

/// Target type for permission overwrites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverwriteType {
    /// Overwrite applies to a role
    Role,
    /// Overwrite applies to a member
    Member,
}

/// Permission overwrite for a channel.
/// Allows per-channel allow/deny for roles and members.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionOverwrite {
    /// ID of the role or member
    pub id: Snowflake,
    /// Type of target (role or member)
    pub target_type: OverwriteType,
    /// Permissions explicitly allowed
    pub allow: Permissions,
    /// Permissions explicitly denied
    pub deny: Permissions,
}

// Permission computation moved to permission_compute module
pub use crate::permission_compute::compute_permissions;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_permissions() {
        let perms = Permissions::default_everyone();
        assert!(perms.contains(Permissions::VIEW_CHANNEL));
        assert!(perms.contains(Permissions::SEND_MESSAGES));
        assert!(perms.contains(Permissions::READ_MESSAGE_HISTORY));
        assert!(perms.contains(Permissions::CONNECT));
        assert!(perms.contains(Permissions::SPEAK));
        assert!(!perms.contains(Permissions::ADMINISTRATOR));
        assert!(!perms.contains(Permissions::MANAGE_CHANNELS));
    }

    #[test]
    fn test_all_permissions() {
        let perms = Permissions::all_permissions();
        assert!(perms.contains(Permissions::ADMINISTRATOR));
        assert!(perms.contains(Permissions::SEND_MESSAGES));
        assert!(perms.contains(Permissions::MANAGE_CHANNELS));
        assert!(perms.contains(Permissions::MANAGE_ROLES));
    }

    #[test]
    fn test_permissions_serialization() {
        let perms = Permissions::SEND_MESSAGES | Permissions::MANAGE_CHANNELS;
        let json = serde_json::to_string(&perms).unwrap();
        let deserialized: Permissions = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, perms);
    }

    #[test]
    fn test_overwrite_type_serialization() {
        let ow_type = OverwriteType::Role;
        let json = serde_json::to_string(&ow_type).unwrap();
        let deserialized: OverwriteType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ow_type);
    }
}
