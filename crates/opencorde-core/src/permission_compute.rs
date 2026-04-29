//! # Permission Computation
//! Functions for computing final permissions based on role overwrites.

use crate::permissions::{OverwriteType, PermissionOverwrite, Permissions};
use crate::snowflake::Snowflake;

/// Computes the final permissions for a target in a channel.
///
/// Algorithm:
/// 1. If target has ADMINISTRATOR, return all permissions
/// 2. Start with base permissions
/// 3. Apply @everyone overwrite, if provided
/// 4. Aggregate matching role overwrites: all denies, then all allows
/// 5. Apply member overwrite if exists
/// 6. Return computed permissions
pub fn compute_permissions(
    base: Permissions,
    overwrites: &[PermissionOverwrite],
    target_id: Snowflake,
    role_ids: &[Snowflake],
    everyone_role_id: Option<Snowflake>,
) -> Permissions {
    // If base has administrator, grant everything
    if base.contains(Permissions::ADMINISTRATOR) {
        return Permissions::all_permissions();
    }

    let mut perms = base;

    // Apply @everyone overwrite first. In Discord this overwrite target is the
    // guild/server ID, represented separately from member role IDs here.
    if let Some(everyone_id) = everyone_role_id {
        for overwrite in overwrites {
            if overwrite.target_type == OverwriteType::Role && overwrite.id == everyone_id {
                perms.remove(overwrite.deny);
                perms.insert(overwrite.allow);
                break;
            }
        }
    }

    // Aggregate role overwrites: all denies are applied before all allows.
    let mut role_deny = Permissions::empty();
    let mut role_allow = Permissions::empty();
    for overwrite in overwrites {
        if overwrite.target_type == OverwriteType::Role
            && role_ids.contains(&overwrite.id)
            && Some(overwrite.id) != everyone_role_id
        {
            role_deny.insert(overwrite.deny);
            role_allow.insert(overwrite.allow);
        }
    }
    perms.remove(role_deny);
    perms.insert(role_allow);

    // Apply member overwrite if exists
    for overwrite in overwrites {
        if overwrite.target_type == OverwriteType::Member && overwrite.id == target_id {
            perms.remove(overwrite.deny);
            perms.insert(overwrite.allow);
        }
    }

    perms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_override() {
        let admin_base = Permissions::ADMINISTRATOR;
        let mut overwrites = Vec::new();
        overwrites.push(PermissionOverwrite {
            id: Snowflake::new(123),
            target_type: OverwriteType::Member,
            allow: Permissions::empty(),
            deny: Permissions::all_permissions(),
        });

        let result = compute_permissions(
            admin_base,
            &overwrites,
            Snowflake::new(123),
            &[Snowflake::new(456)],
            None,
        );

        // Admin should grant all perms regardless of overwrites
        assert_eq!(result, Permissions::all_permissions());
    }

    #[test]
    fn test_compute_permissions_no_overwrites() {
        let base = Permissions::SEND_MESSAGES | Permissions::VIEW_CHANNEL;
        let result = compute_permissions(base, &[], Snowflake::new(999), &[], None);
        assert_eq!(result, base);
    }

    #[test]
    fn test_role_overwrite_allow() {
        let role_id = Snowflake::new(100);
        let user_id = Snowflake::new(200);
        let base = Permissions::SEND_MESSAGES;

        let overwrites = vec![PermissionOverwrite {
            id: role_id,
            target_type: OverwriteType::Role,
            allow: Permissions::MANAGE_MESSAGES,
            deny: Permissions::empty(),
        }];

        let result = compute_permissions(base, &overwrites, user_id, &[role_id], None);
        assert!(result.contains(Permissions::SEND_MESSAGES));
        assert!(result.contains(Permissions::MANAGE_MESSAGES));
    }

    #[test]
    fn test_role_overwrite_deny() {
        let role_id = Snowflake::new(100);
        let user_id = Snowflake::new(200);
        let base = Permissions::SEND_MESSAGES | Permissions::VIEW_CHANNEL;

        let overwrites = vec![PermissionOverwrite {
            id: role_id,
            target_type: OverwriteType::Role,
            allow: Permissions::empty(),
            deny: Permissions::SEND_MESSAGES,
        }];

        let result = compute_permissions(base, &overwrites, user_id, &[role_id], None);
        assert!(!result.contains(Permissions::SEND_MESSAGES));
        assert!(result.contains(Permissions::VIEW_CHANNEL));
    }

    #[test]
    fn test_member_overwrite_precedence() {
        let role_id = Snowflake::new(100);
        let user_id = Snowflake::new(200);
        let base = Permissions::SEND_MESSAGES;

        let overwrites = vec![
            PermissionOverwrite {
                id: role_id,
                target_type: OverwriteType::Role,
                allow: Permissions::empty(),
                deny: Permissions::SEND_MESSAGES,
            },
            PermissionOverwrite {
                id: user_id,
                target_type: OverwriteType::Member,
                allow: Permissions::SEND_MESSAGES,
                deny: Permissions::empty(),
            },
        ];

        let result = compute_permissions(base, &overwrites, user_id, &[role_id], None);
        // Member overwrite should allow it again
        assert!(result.contains(Permissions::SEND_MESSAGES));
    }

    #[test]
    fn test_role_overwrites_are_aggregated() {
        let deny_role = Snowflake::new(100);
        let allow_role = Snowflake::new(101);
        let user_id = Snowflake::new(200);
        let base = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES;

        let overwrites = vec![
            PermissionOverwrite {
                id: allow_role,
                target_type: OverwriteType::Role,
                allow: Permissions::SEND_MESSAGES,
                deny: Permissions::empty(),
            },
            PermissionOverwrite {
                id: deny_role,
                target_type: OverwriteType::Role,
                allow: Permissions::empty(),
                deny: Permissions::SEND_MESSAGES,
            },
        ];

        let result =
            compute_permissions(base, &overwrites, user_id, &[deny_role, allow_role], None);
        assert!(result.contains(Permissions::SEND_MESSAGES));
    }

    #[test]
    fn test_everyone_precedence_before_role_overwrites() {
        let everyone_id = Snowflake::new(1);
        let role_id = Snowflake::new(100);
        let user_id = Snowflake::new(200);
        let base = Permissions::VIEW_CHANNEL;

        let overwrites = vec![
            PermissionOverwrite {
                id: everyone_id,
                target_type: OverwriteType::Role,
                allow: Permissions::SEND_MESSAGES,
                deny: Permissions::empty(),
            },
            PermissionOverwrite {
                id: role_id,
                target_type: OverwriteType::Role,
                allow: Permissions::empty(),
                deny: Permissions::SEND_MESSAGES,
            },
        ];

        let result = compute_permissions(base, &overwrites, user_id, &[role_id], Some(everyone_id));
        assert!(!result.contains(Permissions::SEND_MESSAGES));
    }
}
