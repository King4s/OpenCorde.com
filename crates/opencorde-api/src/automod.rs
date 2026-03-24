//! # AutoMod
//! Checks message content against server keyword rules.
//!
//! ## Usage
//! Call `check_message` before persisting a message to determine if it should be blocked.

use opencorde_core::snowflake::Snowflake;
use opencorde_db::repos::automod_repo;
use sqlx::PgPool;

/// Result of an AutoMod check.
#[derive(Debug)]
pub enum AutomodResult {
    /// Message is allowed.
    Allow,
    /// Message was blocked by a rule. Contains rule name and action.
    Block { rule_name: String, action: String },
}

/// Check message content against all enabled AutoMod rules for a server.
///
/// Returns Allow if no rules match, Block if content contains a banned keyword.
#[tracing::instrument(skip(pool, content))]
pub async fn check_message(
    pool: &PgPool,
    server_id: Snowflake,
    content: &str,
) -> AutomodResult {
    let rules = match automod_repo::list_enabled_by_server(pool, server_id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "failed to fetch automod rules");
            return AutomodResult::Allow;
        }
    };

    let content_lower = content.to_lowercase();

    for rule in rules {
        for keyword in rule.keywords.split(',') {
            let kw = keyword.trim().to_lowercase();
            if kw.is_empty() {
                continue;
            }
            if content_lower.contains(&kw) {
                tracing::info!(
                    rule = %rule.name,
                    keyword = %kw,
                    action = %rule.action,
                    "automod rule triggered"
                );
                return AutomodResult::Block {
                    rule_name: rule.name,
                    action: rule.action,
                };
            }
        }
    }

    AutomodResult::Allow
}
