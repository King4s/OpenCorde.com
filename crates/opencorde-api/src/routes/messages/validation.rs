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

/// Parse `<@USER_ID>` mention tokens from message content.
///
/// Returns a deduplicated list of user Snowflake IDs (as i64) found in the text.
/// Unknown or unparseable IDs are silently skipped.
/// Used by the message send handler to dispatch push notifications to mentioned users.
pub fn extract_mention_ids(content: &str) -> Vec<i64> {
    let mut ids = Vec::new();
    let mut remaining = content;
    while let Some(start) = remaining.find("<@") {
        let after = &remaining[start + 2..];
        if let Some(end) = after.find('>') {
            let id_str = &after[..end];
            if let Ok(id) = id_str.trim().parse::<i64>() {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
        }
        remaining = &remaining[start + 2..];
    }
    ids
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
        assert_eq!(extract_mention_ids("no mentions here"), vec![]);
        // Deduplication
        let ids = extract_mention_ids("<@100> and <@100> again");
        assert_eq!(ids, vec![100_i64]);
        // Multiple distinct mentions
        let ids = extract_mention_ids("<@1> hey <@2>");
        assert_eq!(ids, vec![1_i64, 2_i64]);
    }
}
