//! # Domain Models
//! One model per file. Each defines a primary struct + serialization.

pub mod channel;
pub mod invite;
pub mod member;
pub mod message;
pub mod role;
pub mod server;
pub mod user;
pub mod voice_state;

// Re-exports for convenience
pub use channel::{Channel, ChannelType};
pub use invite::Invite;
pub use member::Member;
pub use message::{Attachment, Message};
pub use role::Role;
pub use server::Server;
pub use user::{User, UserProfile, UserStatus};
pub use voice_state::VoiceState;
