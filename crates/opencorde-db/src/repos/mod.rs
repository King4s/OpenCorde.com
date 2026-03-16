//! # Repositories
//! One repository per entity. Each provides async CRUD operations via PgPool.
//!
//! Each module exports functions as `async fn(pool: &PgPool, ...) -> Result<T, sqlx::Error>`.
//! All functions are instrumented with tracing for structured logging.

pub mod channel_repo;
pub mod invite_repo;
pub mod member_repo;
pub mod message_repo;
pub mod role_repo;
pub mod server_repo;
pub mod user_repo;
