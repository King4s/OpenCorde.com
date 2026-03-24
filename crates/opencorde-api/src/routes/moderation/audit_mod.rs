//! # Audit Logging for Moderation Actions

use crate::AppState;
use opencorde_core::SnowflakeGenerator;
use opencorde_db::repos::{audit_repo, user_repo};

/// Log a moderation action with actor information.
#[tracing::instrument(skip(state))]
pub async fn log_mod_action(
    state: &AppState,
    server_id: opencorde_core::Snowflake,
    actor_id: opencorde_core::Snowflake,
    action: &str,
    target_id: i64,
) {
    let actor_user = user_repo::get_by_id(&state.db, actor_id)
        .await
        .ok()
        .flatten();
    let actor_username = actor_user
        .map(|u| u.username)
        .unwrap_or_else(|| "unknown".to_string());

    let mut audit_generator = SnowflakeGenerator::new(5, 0);
    audit_repo::log_action(
        &state.db,
        audit_generator.next_id(),
        server_id,
        actor_id,
        &actor_username,
        action,
        Some(target_id),
        Some("user"),
        None,
    )
    .await
    .ok();
}
