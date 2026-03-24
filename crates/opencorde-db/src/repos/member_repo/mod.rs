//! # Repository: Server Members
//! CRUD operations for server membership and member roles.
//!
//! Manages the join relationship between users and servers, plus role assignments.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake

mod crud;
mod roles;

pub use crud::{
    MemberRow, MemberWithUsernameRow,
    list_with_usernames_by_server, add_member, remove_member, get_member,
    list_by_server, update_nickname,
};
pub use roles::{
    MemberRoleRow, add_role, remove_role, list_member_roles,
};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_member_row_creation() {
        let now = Utc::now();
        let row = MemberRow {
            user_id: 111222333,
            server_id: 555666777,
            nickname: Some("TestNick".to_string()),
            joined_at: now,
        };

        assert_eq!(row.user_id, 111222333);
        assert_eq!(row.server_id, 555666777);
        assert_eq!(row.nickname, Some("TestNick".to_string()));
    }

    #[test]
    fn test_member_role_row_creation() {
        let row = MemberRoleRow {
            user_id: 111222333,
            server_id: 555666777,
            role_id: 999888777,
        };

        assert_eq!(row.user_id, 111222333);
        assert_eq!(row.role_id, 999888777);
    }
}
