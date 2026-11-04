//! # Message Validation
//! Content validation and ID parsing for message operations.
//!
//! ## Depends On
//! - crate::error::ApiError
//! - opencorde_core::Snowflake

use crate::error::ApiError;
use opencorde_core::Snowflake;

/// Minimum allowed message content length.
const MIN_CONTENT_LENGTH: usize = 1;

/// Maximum allowed message content length.
const MAX_CONTENT_LENGTH: usize = 4000;

/// Validate message content length.
///
/// Returns an error if content is empty or exceeds 4000 characters.
pub fn validate_content(content: &str) -> Result<(), ApiError> {
    let len = content.len();

    if len < MIN_CONTENT_LENGTH {
        tracing::debug!("message content is empty");
        return Err(ApiError::BadRequest(
            "message content cannot be empty".to_string(),
        ));
    }

    if len > MAX_CONTENT_LENGTH {
        tracing::debug!(content_len = len, "message content too long");
        return Err(ApiError::BadRequest(format!(
            "message content cannot exceed {} characters",
            MAX_CONTENT_LENGTH
        )));
    }

    Ok(())
}

/// Parse and validate a Snowflake ID from a string.
pub fn parse_snowflake_id(s: &str) -> Result<Snowflake, ApiError> {
    s.parse::<i64>().map(Snowflake::new).map_err(|_| {
        tracing::debug!(id = s, "failed to parse snowflake id");
        ApiError::BadRequest("invalid id format".into())
    })
}

/// Validate and parse message limit (1-100, defaults to 50).
pub fn validate_limit(limit: Option<i64>) -> i64 {
    match limit {
        Some(l) if (1..=100).contains(&l) => l,
        Some(l) => {
            tracing::debug!(requested = l, "limit out of range, using default");
            50
        }
        None => 50,
    }
}

/// Parsed mention information extracted from message content.
///
/// Supports four Discord-compatible mention types:
/// - `<@USER_ID>` — direct user mentions
/// - `<@&ROLE_ID>` — role mentions (the `&` prefix distinguishes from user IDs)
/// - `@everyone` — literal string triggers mass mention
/// - `@here` — literal string triggers online-only mass mention
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MentionSet {
    /// Unique user Snowflake IDs from `<@ID>` tokens
    pub user_ids: Vec<i64>,
    /// Unique role Snowflake IDs from `<@&ID>` tokens
    pub role_ids: Vec<i64>,
    /// Whether the message contains the literal `@everyone`
    pub has_everyone: bool,
    /// Whether the message contains the literal `@here`
    pub has_here: bool,
}

impl MentionSet {
    /// Returns `true` when no mentions of any type were found.
    pub fn is_empty(&self) -> bool {
        self.user_ids.is_empty() && self.role_ids.is_empty() && !self.has_everyone && !self.has_here
    }
}

/// Parse all mention types from message content.
///
/// Detects literal `@everyone` / `@here` strings and extracts IDs from
/// `<@USER_ID>` and `<@&ROLE_ID>` tokens. Duplicate IDs are silently collapsed.
/// Unparseable tokens are ignored.
pub fn extract_mentions(content: &str) -> MentionSet {
    let mut user_ids = Vec::new();
    let mut role_ids = Vec::new();

    let has_everyone = content.contains("@everyone");
    let has_here = content.contains("@here");

    let mut remaining = content;
    while let Some(start) = remaining.find("<@") {
        let after = &remaining[start + 2..];
        if let Some(end) = after.find('>') {
            let inner = &after[..end];
            if let Some(role_inner) = inner.strip_prefix('&') {
                if let Ok(id) = role_inner.trim().parse::<i64>() {
                    if !role_ids.contains(&id) {
                        role_ids.push(id);
                    }
                }
            } else if let Ok(id) = inner.trim().parse::<i64>() {
                if !user_ids.contains(&id) {
                    user_ids.push(id);
                }
            }
        }
        remaining = &remaining[start + 2..];
    }

    MentionSet {
        user_ids,
        role_ids,
        has_everyone,
        has_here,
    }
}

/// Parse `<@USER_ID>` mention tokens from message content.
///
/// Convenience wrapper around [`extract_mentions`] that returns only user IDs.
/// For full mention parsing (including @everyone, @here, and role mentions),
/// prefer [`extract_mentions`].
pub fn extract_mention_ids(content: &str) -> Vec<i64> {
    extract_mentions(content).user_ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_content_valid() {
        assert!(validate_content("Hello").is_ok());
        assert!(validate_content("A").is_ok());
        assert!(validate_content(&"x".repeat(4000)).is_ok());
    }

    #[test]
    fn test_validate_content_empty() {
        assert!(validate_content("").is_err());
    }

    #[test]
    fn test_validate_content_too_long() {
        let long_content = "x".repeat(4001);
        assert!(validate_content(&long_content).is_err());
    }

    #[test]
    fn test_parse_snowflake_id_valid() {
        let result = parse_snowflake_id("123456789");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_i64(), 123456789);
    }

    #[test]
    fn test_parse_snowflake_id_invalid() {
        assert!(parse_snowflake_id("not_a_number").is_err());
        assert!(parse_snowflake_id("").is_err());
    }

    #[test]
    fn test_validate_limit_valid() {
        assert_eq!(validate_limit(Some(50)), 50);
        assert_eq!(validate_limit(Some(1)), 1);
        assert_eq!(validate_limit(Some(100)), 100);
    }

    #[test]
    fn test_validate_limit_default() {
        assert_eq!(validate_limit(None), 50);
    }

    #[test]
    fn test_validate_limit_out_of_range() {
        assert_eq!(validate_limit(Some(0)), 50);
        assert_eq!(validate_limit(Some(101)), 50);
        assert_eq!(validate_limit(Some(1000)), 50);
    }

    #[test]
    fn test_extract_mention_ids() {
        assert_eq!(extract_mention_ids("hello <@123> world"), vec![123_i64]);
        assert_eq!(extract_mention_ids("no mentions here"), Vec::<i64>::new());
        // Deduplication
        let ids = extract_mention_ids("<@100> and <@100> again");
        assert_eq!(ids, vec![100_i64]);
        // Multiple distinct mentions
        let ids = extract_mention_ids("<@1> hey <@2>");
        assert_eq!(ids, vec![1_i64, 2_i64]);
    }

    #[test]
    fn test_extract_mentions_users_only() {
        let m = extract_mentions("hello <@123> and <@456>");
        assert_eq!(m.user_ids, vec![123_i64, 456_i64]);
        assert!(m.role_ids.is_empty());
        assert!(!m.has_everyone);
        assert!(!m.has_here);
        assert!(!m.is_empty());
    }

    #[test]
    fn test_extract_mentions_everyone() {
        let m = extract_mentions("@everyone check this out");
        assert!(m.has_everyone);
        assert!(!m.has_here);
        assert!(m.user_ids.is_empty());
        assert!(m.role_ids.is_empty());
        assert!(!m.is_empty());
    }

    #[test]
    fn test_extract_mentions_here() {
        let m = extract_mentions("@here meeting starting");
        assert!(!m.has_everyone);
        assert!(m.has_here);
        assert!(m.user_ids.is_empty());
        assert!(m.role_ids.is_empty());
    }

    #[test]
    fn test_extract_mentions_roles() {
        let m = extract_mentions("hey <@&111> and <@&222> listen up");
        assert_eq!(m.role_ids, vec![111_i64, 222_i64]);
        assert!(m.user_ids.is_empty());
        assert!(!m.has_everyone);
    }

    #[test]
    fn test_extract_mentions_mixed() {
        let m = extract_mentions("@everyone <@1> <@&5> <@2> @here");
        assert!(m.has_everyone);
        assert!(m.has_here);
        assert_eq!(m.user_ids, vec![1_i64, 2_i64]);
        assert_eq!(m.role_ids, vec![5_i64]);
    }

    #[test]
    fn test_extract_mentions_empty() {
        let m = extract_mentions("no mentions at all");
        assert!(m.is_empty());
    }
}
