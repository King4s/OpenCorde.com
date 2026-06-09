//! Bot gateway — Discord-compatible intents, opcode envelope, identify/ready payloads.

use serde::{Deserialize, Serialize};

/// Gateway intents bitfield for bot event subscriptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GatewayIntents(pub i64);

impl GatewayIntents {
    pub const GUILDS: i64 = 1 << 0;
    pub const GUILD_MEMBERS: i64 = 1 << 1;
    pub const GUILD_MODERATION: i64 = 1 << 2;
    pub const GUILD_MESSAGES: i64 = 1 << 9;
    pub const GUILD_MESSAGE_REACTIONS: i64 = 1 << 10;
    pub const GUILD_VOICE_STATES: i64 = 1 << 7;
    pub const GUILD_PRESENCES: i64 = 1 << 8;
    pub const DIRECT_MESSAGES: i64 = 1 << 12;
    pub const MESSAGE_CONTENT: i64 = 1 << 15;
    pub const INTEGRATIONS: i64 = 1 << 20;

    pub const fn new(bits: i64) -> Self { Self(bits) }
    pub fn contains(&self, intent: i64) -> bool { self.0 & intent != 0 }
    pub fn bits(&self) -> i64 { self.0 }
}

impl Default for GatewayIntents {
    fn default() -> Self { Self(Self::GUILDS | Self::GUILD_MESSAGES) }
}

/// Gateway opcode envelope (Discord-compatible).
#[derive(Debug, Serialize, Deserialize)]
pub struct GatewayPayload<T = serde_json::Value> {
    pub op: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub d: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t: Option<String>,
}

impl GatewayPayload {
    pub const DISPATCH: u8 = 0;
    pub const HEARTBEAT: u8 = 1;
    pub const IDENTIFY: u8 = 2;
    pub const HELLO: u8 = 10;
    pub const HEARTBEAT_ACK: u8 = 11;
    pub const RECONNECT: u8 = 7;
    pub const INVALID_SESSION: u8 = 9;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdentifyPayload {
    pub token: String,
    pub intents: i64,
    pub properties: IdentifyProperties,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdentifyProperties {
    pub os: String,
    pub library: String,
}

#[derive(Debug, Serialize)]
pub struct HelloPayload {
    pub heartbeat_interval: u32,
}

#[derive(Debug, Serialize)]
pub struct ReadyPayload {
    pub application_id: i64,
    pub bot_user_id: i64,
    pub session_id: String,
}
