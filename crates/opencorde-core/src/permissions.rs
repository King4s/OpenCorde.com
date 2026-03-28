//! # Permission Bitfield
//! 64-bit permission system using Discord's exact bit layout.
//!
//! Bit values are identical to Discord's permission flag spec so that
//! the client (permissions.ts) and server agree on every flag's meaning.
//! Reference: https://docs.discord.com/developers/topics/permissions
//!
//! ## Depends On
//! - bitflags (efficient bitset type)
//! - serde (JSON serialization)

use crate::snowflake::Snowflake;
use bitflags::bitflags;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

bitflags! {
    /// Permission flags for roles and channels.
    /// Bit positions match Discord's permission flag spec exactly.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Permissions: u64 {
        // General server permissions
        const CREATE_INVITE         = 1 << 0;
        const KICK_MEMBERS          = 1 << 1;
        const BAN_MEMBERS           = 1 << 2;
        const ADMINISTRATOR         = 1 << 3;
        const MANAGE_CHANNELS       = 1 << 4;
        const MANAGE_SERVER         = 1 << 5;
        const ADD_REACTIONS         = 1 << 6;
        const VIEW_AUDIT_LOG        = 1 << 7;
        const PRIORITY_SPEAKER      = 1 << 8;
        const STREAM                = 1 << 9;
        const VIEW_CHANNEL          = 1 << 10;
        const SEND_MESSAGES         = 1 << 11;
        const SEND_TTS_MESSAGES     = 1 << 12;
        const MANAGE_MESSAGES       = 1 << 13;
        const EMBED_LINKS           = 1 << 14;
        const ATTACH_FILES          = 1 << 15;
        const READ_MESSAGE_HISTORY  = 1 << 16;
        const MENTION_EVERYONE      = 1 << 17;
        const USE_EXTERNAL_EMOJIS   = 1 << 18;
        const VIEW_GUILD_INSIGHTS   = 1 << 19;
        // Voice permissions
        const CONNECT               = 1 << 20;
        const SPEAK                 = 1 << 21;
        const MUTE_MEMBERS          = 1 << 22;
        const DEAFEN_MEMBERS        = 1 << 23;
        const MOVE_MEMBERS          = 1 << 24;
        const USE_VAD               = 1 << 25;
        // Member management
        const CHANGE_NICKNAME       = 1 << 26;
        const MANAGE_NICKNAMES      = 1 << 27;
        const MANAGE_ROLES          = 1 << 28;
        const MANAGE_WEBHOOKS       = 1 << 29;
        const MANAGE_GUILD_EXPRESSIONS = 1 << 30;
        const USE_APPLICATION_COMMANDS = 1 << 31;
        // Stage / events / threads
        const REQUEST_TO_SPEAK      = 1 << 32;
        const MANAGE_EVENTS         = 1 << 33;
        const MANAGE_THREADS        = 1 << 34;
        const CREATE_PUBLIC_THREADS = 1 << 35;
        const CREATE_PRIVATE_THREADS = 1 << 36;
        const USE_EXTERNAL_STICKERS = 1 << 37;
        const SEND_MESSAGES_IN_THREADS = 1 << 38;
        // Moderation
        const MODERATE_MEMBERS      = 1 << 40;
        const CREATE_EVENTS         = 1 << 44;
        const SEND_VOICE_MESSAGES   = 1 << 46;
        const SEND_POLLS            = 1 << 49;
        const PIN_MESSAGES          = 1 << 51;
        const BYPASS_SLOWMODE       = 1 << 52;
    }
}

impl Permissions {
    /// Default permissions for the @everyone role on a new server.
    pub fn default_everyone() -> Self {
        Permissions::CREATE_INVITE
            | Permissions::VIEW_CHANNEL
            | Permissions::SEND_MESSAGES
            | Permissions::EMBED_LINKS
            | Permissions::ATTACH_FILES
            | Permissions::READ_MESSAGE_HISTORY
            | Permissions::ADD_REACTIONS
            | Permissions::CONNECT
            | Permissions::SPEAK
            | Permissions::USE_VAD
            | Permissions::CHANGE_NICKNAME
    }

    /// All permissions set (used for Administrator bypass).
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

// Permission computation is in permission_compute module
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
    fn test_administrator_bit_is_discord_compatible() {
        // ADMINISTRATOR must be bit 3 to match Discord's layout
        assert_eq!(Permissions::ADMINISTRATOR.bits(), 1 << 3);
    }

    #[test]
    fn test_send_messages_bit_is_discord_compatible() {
        assert_eq!(Permissions::SEND_MESSAGES.bits(), 1 << 11);
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
