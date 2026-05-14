//! # Repository: Stage Channels
//! Manages stage sessions and participant roles.
//!
//! Provides functions for:
//! - Starting/ending stage sessions
//! - Managing speaker and audience roles
//! - Tracking hand-raised requests to speak
//!
//! ## Depends On
//! - sqlx (database driver)
//! - opencorde_core::Snowflake
//! - chrono (DateTime handling)

mod participant;
mod session;

pub use participant::{
    StageParticipantRow, demote_to_audience, join_stage, leave_stage, list_participants,
    lower_hand, promote_to_speaker, raise_hand,
};
pub use session::{StageSessionRow, end_session, get_session, start_session};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_stage_session_row_creation() {
        let row = StageSessionRow {
            id: 111,
            channel_id: 222,
            topic: Some("discussion".to_string()),
            started_by: 333,
            started_at: Utc::now(),
        };

        assert_eq!(row.id, 111);
        assert_eq!(row.channel_id, 222);
        assert_eq!(row.topic, Some("discussion".to_string()));
        assert_eq!(row.started_by, 333);
    }

    #[test]
    fn test_stage_participant_row_creation() {
        let row = StageParticipantRow {
            id: 444,
            channel_id: 222,
            user_id: 555,
            username: "alice".to_string(),
            role: "speaker".to_string(),
            hand_raised: false,
            joined_at: Utc::now(),
        };

        assert_eq!(row.id, 444);
        assert_eq!(row.user_id, 555);
        assert_eq!(row.username, "alice");
        assert_eq!(row.role, "speaker");
        assert!(!row.hand_raised);
    }

    #[test]
    fn test_stage_participant_audience() {
        let row = StageParticipantRow {
            id: 666,
            channel_id: 222,
            user_id: 777,
            username: "bob".to_string(),
            role: "audience".to_string(),
            hand_raised: true,
            joined_at: Utc::now(),
        };

        assert_eq!(row.role, "audience");
        assert!(row.hand_raised);
    }
}
