//! # Domain Models
//! One model per file. Each defines a primary struct + serialization.

pub mod application;
pub mod app_install;
pub mod bot_user;
pub mod channel;
pub mod invite;
pub mod member;
pub mod message;
pub mod oauth_scope;
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
pub mod oauth_client_secret;
pub mod bot_token;
pub mod application_command;
pub mod message_embed;
