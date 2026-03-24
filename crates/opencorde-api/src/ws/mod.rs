//! # WebSocket Gateway
//! Real-time event delivery via WebSocket following Discord's gateway model.
//!
//! ## Modules
//! - `handler` — Connection lifecycle and event streaming
//! - `events` — Event serialization helpers
//! - `dispatch` — Event dispatch filtering (which events reach which clients)
//!
//! ## Architecture
//! WebSocket connections follow a simple state machine:
//! 1. Server sends HELLO with heartbeat interval
//! 2. Client sends IDENTIFY with JWT token
//! 3. Server validates token and sends READY
//! 4. Periodic heartbeats keep connection alive
//! 5. Events are pushed to connected clients

pub mod dispatch;
pub mod events;
pub mod handler;
