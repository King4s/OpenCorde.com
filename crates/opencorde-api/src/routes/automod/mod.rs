//! # Route: AutoMod
//! Keyword filter rule management for servers.
//!
//! ## Endpoints
//! - POST /api/v1/servers/{server_id}/automod — Create rule
//! - GET /api/v1/servers/{server_id}/automod — List rules
//! - PATCH /api/v1/automod/{rule_id} — Update rule
//! - DELETE /api/v1/automod/{rule_id} — Delete rule

mod handlers;
mod types;

pub use handlers::router;
pub use types::{AutomodRuleResponse, CreateAutomodRuleRequest, UpdateAutomodRuleRequest};
