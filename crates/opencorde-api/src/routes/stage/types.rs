//! # Stage Channel Types
//! Request and response types for stage channel endpoints.
//!
//! ## Depends On
//! - serde, chrono

use serde::{Deserialize, Serialize};

/// Response for a stage session.
#[derive(Debug, Serialize)]
pub struct StageSessionResponse {
    pub id: String,
    pub channel_id: String,
    pub topic: Option<String>,
    pub started_by: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

/// Response for a stage participant.
#[derive(Debug, Serialize)]
pub struct StageParticipantResponse {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub role: String,
    pub hand_raised: bool,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

/// Complete stage detail: session + all participants.
#[derive(Debug, Serialize)]
pub struct StageDetailResponse {
    pub session: StageSessionResponse,
    pub participants: Vec<StageParticipantResponse>,
}

/// Request to start a stage session.
#[derive(Debug, Deserialize)]
pub struct StartStageRequest {
    pub topic: Option<String>,
}

/// Request to raise/lower hand.
#[derive(Debug, Deserialize)]
pub struct HandRequest {
    pub raised: bool,
}

/// Request to promote/demote speaker.
#[derive(Debug, Deserialize)]
pub struct SpeakerRequest {
    pub speaker: bool,
}
