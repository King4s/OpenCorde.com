//! # Gateway Events
//! WebSocket gateway event types for real-time communication.
//!
//! All gateway events use serde's tag + content for clean JSON encoding.
//! Format: `{ "type": "event_name", "data": {...} }`

use crate::models::{Channel, Message, Server, UserProfile, VoiceState};
use crate::models::event::Event;
use crate::snowflake::Snowflake;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Stage session data for gateway events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageSessionData {
    pub id: Snowflake,
    pub channel_id: Snowflake,
    pub topic: Option<String>,
    pub started_by: Snowflake,
    pub started_at: DateTime<Utc>,
}

/// Stage participant data for gateway events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageParticipantData {
    pub id: Snowflake,
    pub channel_id: Snowflake,
    pub user_id: Snowflake,
    pub username: String,
    pub role: String,
    pub hand_raised: bool,
    pub joined_at: DateTime<Utc>,
}

/// Gateway lifecycle and data events.
/// Serializes with `type` field and nested `data` object.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum GatewayEvent {
    // Lifecycle events
    /// Server says hello: client must send Identify
    Hello { heartbeat_interval: u64 },
    /// Client authenticates with token
    Identify { token: String },
    /// Server acknowledges authentication and sends initial state
    Ready {
        user: UserProfile,
        servers: Vec<Server>,
    },
    /// Client heartbeat to keep connection alive
    Heartbeat,
    /// Server acknowledges heartbeat
    HeartbeatAck,

    // Message events
    /// New message posted
    MessageCreate { message: Message },
    /// Message edited
    MessageUpdate { message: Message },
    /// Message deleted
    MessageDelete {
        channel_id: Snowflake,
        message_id: Snowflake,
    },

    // Typing indicator
    /// User started typing in channel
    TypingStart {
        channel_id: Snowflake,
        user_id: Snowflake,
        timestamp: DateTime<Utc>,
    },

    // Presence/status
    /// User's status changed
    PresenceUpdate {
        user_id: Snowflake,
        status: crate::models::UserStatus,
    },

    // Voice events
    /// User connected/disconnected from voice
    VoiceStateUpdate { voice_state: VoiceState },

    // Server events
    /// New server created
    ServerCreate { server: Server },
    /// Server updated
    ServerUpdate { server: Server },
    /// Server deleted
    ServerDelete { server_id: Snowflake },

    // Channel events
    /// New channel created
    ChannelCreate { channel: Channel },
    /// Channel updated
    ChannelUpdate { channel: Channel },
    /// Channel deleted
    ChannelDelete { channel_id: Snowflake },

    // Member events
    /// User joined server
    MemberJoin {
        server_id: Snowflake,
        member: crate::models::Member,
    },
    /// User left server
    MemberLeave {
        server_id: Snowflake,
        user_id: Snowflake,
    },

    // Scheduled event events
    /// New scheduled event created
    EventCreate { event: Event },
    /// Event status updated
    EventUpdate { event: Event },
    /// Event deleted
    EventDelete {
        server_id: Snowflake,
        event_id: Snowflake,
    },
    /// RSVP count changed (user RSVP'd or un-RSVP'd)
    EventRsvpUpdate {
        event_id: Snowflake,
        server_id: Snowflake,
        rsvp_count: i64,
    },

    // Stage channel events
    /// Stage session started
    StageSessionStart { session: StageSessionData },
    /// Stage session ended
    StageSessionEnd {
        channel_id: Snowflake,
    },
    /// Participant joined stage
    StageParticipantJoin { participant: StageParticipantData },
    /// Participant left stage
    StageParticipantLeave {
        channel_id: Snowflake,
        user_id: Snowflake,
    },
    /// Participant updated (role change, hand raise/lower)
    StageParticipantUpdate { participant: StageParticipantData },
}
