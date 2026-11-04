//! # Search Filter Parser
//! Parses Discord-style search filters from query strings.
//!
//! ## Supported Filters
//! - `from:username` or `from:user_id` — filter by author
//! - `mentions:username` — filter by mentioned user
//! - `has:link` — filter messages with URLs
//! - `has:file` — filter messages with attachments
//! - `has:embed` — filter messages with embeds (future)
//! - `before:YYYY-MM-DD` — filter by date before
//! - `after:YYYY-MM-DD` — filter by date after
//! - `in:channel-name` — filter by channel
//! - `pinned:true` — filter pinned messages
//! - `file:type` — filter by attachment type (e.g., png, pdf, image)

use chrono::NaiveDate;

/// Structured search filters parsed from a query string.
#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    /// Filter by author username or ID
    pub from: Option<String>,
    /// Filter by mentioned username or ID
    pub mentions: Option<String>,
    /// Filter by has_link
    pub has_link: Option<bool>,
    /// Filter by has_file
    pub has_file: Option<bool>,
    /// Filter by has_embed (for future use)
    pub has_embed: Option<bool>,
    /// Filter by date before (unix timestamp)
    pub before: Option<i64>,
    /// Filter by date after (unix timestamp)
    pub after: Option<i64>,
    /// Filter by channel name
    pub in_channel: Option<String>,
    /// Filter by pinned status
    pub pinned: Option<bool>,
    /// Filter by attachment content type prefix
    pub file_type: Option<String>,
}

/// Parsed search query with filters stripped from text.
#[derive(Debug, Clone)]
pub struct ParsedQuery {
    /// The cleaned query text (filters removed)
    pub text: String,
    /// Structured filters extracted from the query
    pub filters: SearchFilters,
}

/// Parse a search query string into a cleaned text query and structured filters.
///
/// Filters are removed from the text portion. Unrecognized filter values
/// are left in the text for the full-text search engine to handle.
///
/// # Examples
/// ```
/// # use opencorde_search::filter_parser::{parse_query, ParsedQuery};
/// let ParsedQuery { text, filters } = parse_query("hello from:alice has:link before:2024-06-01");
/// assert_eq!(text, "hello");
/// assert_eq!(filters.from, Some("alice".to_string()));
/// assert_eq!(filters.has_link, Some(true));
/// assert!(filters.before.is_some());
/// ```
pub fn parse_query(raw: &str) -> ParsedQuery {
    let mut filters = SearchFilters::default();
    let mut tokens: Vec<&str> = Vec::new();

    for word in raw.split_whitespace() {
        if let Some(filter) = try_parse_filter(word) {
            match filter {
                ParsedFilter::From(val) => filters.from = Some(val),
                ParsedFilter::Mentions(val) => filters.mentions = Some(val),
                ParsedFilter::HasLink => filters.has_link = Some(true),
                ParsedFilter::HasFile => filters.has_file = Some(true),
                ParsedFilter::HasEmbed => filters.has_embed = Some(true),
                ParsedFilter::Before(ts) => filters.before = Some(ts),
                ParsedFilter::After(ts) => filters.after = Some(ts),
                ParsedFilter::InChannel(val) => filters.in_channel = Some(val),
                ParsedFilter::Pinned => filters.pinned = Some(true),
                ParsedFilter::FileType(val) => filters.file_type = Some(val),
            }
        } else {
            tokens.push(word);
        }
    }

    let text = tokens.join(" ");

    tracing::debug!(
        text = %text,
        ?filters,
        "parsed search query"
    );

    ParsedQuery { text, filters }
}

enum ParsedFilter {
    From(String),
    Mentions(String),
    HasLink,
    HasFile,
    HasEmbed,
    Before(i64),
    After(i64),
    InChannel(String),
    Pinned,
    FileType(String),
}

/// Try to parse a single word as a filter token.
/// Returns None if the word is not a recognized filter.
fn try_parse_filter(word: &str) -> Option<ParsedFilter> {    // Use original-case for filter values
    if let Some(rest) = strip_prefix_ci(word, "from:") {
        if rest.is_empty() {
            return None;
        }
        return Some(ParsedFilter::From(rest.to_string()));
    }

    if let Some(rest) = strip_prefix_ci(word, "mentions:") {
        if rest.is_empty() {
            return None;
        }
        return Some(ParsedFilter::Mentions(rest.to_string()));
    }

    if let Some(rest) = strip_prefix_ci(word, "has:") {
        return match rest.to_lowercase().as_str() {
            "link" => Some(ParsedFilter::HasLink),
            "file" => Some(ParsedFilter::HasFile),
            "embed" => Some(ParsedFilter::HasEmbed),
            _ => None,
        };
    }

    if let Some(rest) = strip_prefix_ci(word, "before:") {
        return parse_date(rest).map(|ts| ParsedFilter::Before(ts));
    }

    if let Some(rest) = strip_prefix_ci(word, "after:") {
        return parse_date(rest).map(|ts| ParsedFilter::After(ts));
    }

    if let Some(rest) = strip_prefix_ci(word, "in:") {
        if rest.is_empty() {
            return None;
        }
        return Some(ParsedFilter::InChannel(rest.to_string()));
    }

    if let Some(rest) = strip_prefix_ci(word, "pinned:") {
        return match rest.to_lowercase().as_str() {
            "true" | "yes" => Some(ParsedFilter::Pinned),
            _ => None,
        };
    }

    if let Some(rest) = strip_prefix_ci(word, "file:") {
        if rest.is_empty() {
            return None;
        }
        return Some(ParsedFilter::FileType(rest.to_string()));
    }

    None
}

/// Strip a prefix case-insensitively, returning the remainder with original case.
fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() < prefix.len() {
        return None;
    }
    if s[..prefix.len()].to_lowercase() == prefix.to_lowercase() {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

/// Parse a date string into Unix timestamp (seconds).
/// Supports YYYY-MM-DD format.
fn parse_date(s: &str) -> Option<i64> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_text() {
        let parsed = parse_query("hello world");
        assert_eq!(parsed.text, "hello world");
        assert!(parsed.filters.from.is_none());
        assert!(parsed.filters.has_link.is_none());
    }

    #[test]
    fn test_parse_from_filter() {
        let parsed = parse_query("hello from:alice");
        assert_eq!(parsed.text, "hello");
        assert_eq!(parsed.filters.from, Some("alice".to_string()));
    }

    #[test]
    fn test_parse_has_filters() {
        let parsed = parse_query("test has:link has:file");
        assert_eq!(parsed.text, "test");
        assert_eq!(parsed.filters.has_link, Some(true));
        assert_eq!(parsed.filters.has_file, Some(true));
    }

    #[test]
    fn test_parse_date_filters() {
        let parsed = parse_query("chat before:2024-06-15 after:2024-01-01");
        assert_eq!(parsed.text, "chat");
        assert!(parsed.filters.before.is_some());
        assert!(parsed.filters.after.is_some());
        // Verify the timestamps are reasonable
        let before = parsed.filters.before.unwrap();
        let after = parsed.filters.after.unwrap();
        assert!(before > after);
    }

    #[test]
    fn test_parse_in_channel() {
        let parsed = parse_query("discussion in:general");
        assert_eq!(parsed.text, "discussion");
        assert_eq!(parsed.filters.in_channel, Some("general".to_string()));
    }

    #[test]
    fn test_parse_pinned() {
        let parsed = parse_query("rules pinned:true");
        assert_eq!(parsed.text, "rules");
        assert_eq!(parsed.filters.pinned, Some(true));
    }

    #[test]
    fn test_parse_file_type() {
        let parsed = parse_query("screenshot file:png");
        assert_eq!(parsed.text, "screenshot");
        assert_eq!(parsed.filters.file_type, Some("png".to_string()));
    }

    #[test]
    fn test_parse_mentions() {
        let parsed = parse_query("attention mentions:bob");
        assert_eq!(parsed.text, "attention");
        assert_eq!(parsed.filters.mentions, Some("bob".to_string()));
    }

    #[test]
    fn test_parse_multiple_filters() {
        let parsed = parse_query(
            "project from:alice has:link before:2024-12-31 in:general pinned:true",
        );
        assert_eq!(parsed.text, "project");
        assert_eq!(parsed.filters.from, Some("alice".to_string()));
        assert_eq!(parsed.filters.has_link, Some(true));
        assert!(parsed.filters.before.is_some());
        assert_eq!(parsed.filters.in_channel, Some("general".to_string()));
        assert_eq!(parsed.filters.pinned, Some(true));
    }

    #[test]
    fn test_empty_filters_ignored() {
        let parsed = parse_query("hello from:");
        assert_eq!(parsed.text, "hello from:");
        assert!(parsed.filters.from.is_none());
    }

    #[test]
    fn test_invalid_has_filter_passed_through() {
        let parsed = parse_query("test has:unknown");
        assert_eq!(parsed.text, "test has:unknown");
        assert!(parsed.filters.has_link.is_none());
    }

    #[test]
    fn test_invalid_date_passed_through() {
        let parsed = parse_query("log before:not-a-date");
        assert_eq!(parsed.text, "log before:not-a-date");
        assert!(parsed.filters.before.is_none());
    }

    #[test]
    fn test_pinned_false_ignored() {
        let parsed = parse_query("stuff pinned:false");
        assert_eq!(parsed.text, "stuff pinned:false");
        assert!(parsed.filters.pinned.is_none());
    }

    #[test]
    fn test_case_insensitive_filters() {
        let parsed = parse_query("TEST From:Alice Has:Link Has:File Before:2024-06-01 After:2024-01-01 In:General Pinned:true Mentions:bob File:png");
        assert_eq!(parsed.text, "TEST");
        assert_eq!(parsed.filters.from, Some("Alice".to_string()));
        assert_eq!(parsed.filters.has_link, Some(true));
        assert_eq!(parsed.filters.has_file, Some(true));
        assert_eq!(parsed.filters.in_channel, Some("General".to_string()));
        assert_eq!(parsed.filters.pinned, Some(true));
        assert_eq!(parsed.filters.mentions, Some("bob".to_string()));
    }
}
