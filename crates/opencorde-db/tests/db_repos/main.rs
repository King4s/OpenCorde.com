//! # Integration Tests: Repository Layer
//! Runs the repo functions against a real, ephemeral Postgres 16 container
//! (testcontainers) with the full migration chain applied.
//!
//! One container is started per test binary and shared by all modules; each
//! module claims a disjoint snowflake ID range so parallel tests never
//! collide on rows. Requires a running Docker daemon — the same requirement
//! as the docker-compose dev environment.

mod common;

mod bot_tokens;
mod dm;
mod embeds;
mod events;
mod integration_logs;
mod interactions;
