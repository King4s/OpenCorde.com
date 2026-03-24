//! # Discord Bridge Module
//! Bidirectional message bridge between Discord and OpenCorde.
//!
//! ## Submodules
//! - `gateway` — Discord WebSocket gateway event loop (Discord → OpenCorde)
//! - `api`     — Discord REST API client (OpenCorde → Discord via webhook)
//! - `mapper`  — Channel mapping DB queries and message insertion
//! - `puppet`  — Ghost user lifecycle management

pub mod api;
pub mod gateway;
pub mod mapper;
pub mod puppet;

pub use api::DiscordApi;
