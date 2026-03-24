//! # Repositories
//! One repository per entity. Each provides async CRUD operations via PgPool.
//!
//! Each module exports functions as `async fn(pool: &PgPool, ...) -> Result<T, sqlx::Error>`.
//! All functions are instrumented with tracing for structured logging.

pub mod audit_repo;
pub mod automod_repo;
pub mod ban_repo;
pub mod channel_override_repo;
pub mod channel_repo;
pub mod dm_repo;
pub mod emoji_repo;
pub mod event_repo;
pub mod forum_repo;
pub mod invite_repo;
pub mod member_repo;
pub mod mesh_peer_repo;
pub mod message_repo;
pub mod pin_repo;
pub mod reaction_repo;
pub mod read_state_repo;
pub mod relationship_repo;
pub mod role_repo;
pub mod server_repo;
pub mod slash_command_repo;
pub mod stage_repo;
pub mod thread_repo;
pub mod user_repo;
pub mod voice_state_repo;
pub mod webhook_repo;
