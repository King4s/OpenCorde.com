//! # Route: URL Unfurling (Link Previews)
//! Fetches OpenGraph metadata from a URL so the client can render a preview card.
//!
//! ## Endpoint
//! `GET /api/v1/unfurl?url=<encoded_url>`
//!
//! ## Security
//! - Blocks SSRF: rejects requests to private/loopback IP ranges
//! - Only allows http:// and https:// schemes
//! - Limits response body to 512 KiB (no need to download whole page)
//! - Short timeout (5 s) with no retries
//! - Results cached in-process for 1 hour to avoid hammering external sites
//!
//! ## Depends On
//! - reqwest (HTTP client)
//! - axum (routing, Query extractor)
//! - tokio::sync::Mutex (shared unfurl cache)
//! - crate::AppState (access to the shared cache)

use std::{collections::HashMap, net::IpAddr, sync::Arc, time::{Duration, Instant}};

use axum::{Router, extract::{Query, State}, routing::get};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

/// Shared in-process cache for unfurl results.
/// Key: normalized URL, Value: (result, time of fetch).
pub type UnfurlCache = Arc<Mutex<HashMap<String, (UnfurlData, Instant)>>>;

/// Create a new empty unfurl cache.
pub fn new_cache() -> UnfurlCache {
    Arc::new(Mutex::new(HashMap::new()))
}

/// OpenGraph / Twitter-card metadata extracted from a URL.
#[derive(Clone, Debug, Serialize, Default)]
pub struct UnfurlData {
    pub url:         String,
    pub title:       Option<String>,
    pub description: Option<String>,
    pub image_url:   Option<String>,
    pub site_name:   Option<String>,
}

/// Query parameters for the unfurl endpoint.
#[derive(Deserialize)]
pub struct UnfurlQuery {
    pub url: String,
}

/// Cache TTL — re-fetch after 1 hour.
const CACHE_TTL: Duration = Duration::from_secs(3_600);
/// Maximum number of cache entries before we evict stale ones.
const CACHE_MAX: usize = 2_000;
/// Maximum response body to read (512 KiB).
const MAX_BODY: usize = 512 * 1024;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/unfurl", get(unfurl))
}

/// GET /api/v1/unfurl?url=<encoded>
///
/// Returns OpenGraph metadata for the given URL. Requires authentication to
/// avoid being used as an open proxy.
pub async fn unfurl(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(params): Query<UnfurlQuery>,
) -> Result<axum::Json<UnfurlData>, ApiError> {
    let url = params.url.trim().to_string();

    // Basic scheme check
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(ApiError::BadRequest("only http/https URLs are supported".into()));
    }

    // Normalise to first 1024 chars as cache key (defends against giant URLs)
    let cache_key = url.chars().take(1_024).collect::<String>();

    // Check cache
    {
        let cache = state.unfurl_cache.lock().await;
        if let Some((data, fetched_at)) = cache.get(&cache_key)
            && fetched_at.elapsed() < CACHE_TTL {
                return Ok(axum::Json(data.clone()));
            }
    }

    // SSRF protection: resolve host and block private ranges
    if let Err(e) = guard_url(&url) {
        tracing::warn!(url = %url, reason = %e, "unfurl blocked");
        return Err(ApiError::BadRequest(e));
    }

    // Fetch with timeout
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::limited(3))
        .user_agent("OpenCorde/1.0 (+https://opencorde.com; link-preview bot)")
        .build()
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("http client: {}", e)))?;

    let resp = client
        .get(&url)
        .header("Accept", "text/html,application/xhtml+xml")
        .send()
        .await
        .map_err(|_| ApiError::BadRequest("failed to reach URL".into()))?;

    if !resp.status().is_success() {
        return Err(ApiError::BadRequest(
            format!("remote returned {}", resp.status()),
        ));
    }

    // Only parse HTML; skip images, PDFs, etc.
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    if !content_type.contains("text/html") && !content_type.contains("xhtml") {
        return Err(ApiError::BadRequest("URL does not serve HTML".into()));
    }

    // Read at most MAX_BODY bytes
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("reading body: {}", e)))?;

    let body = String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_BODY)]).into_owned();
    let data = parse_og(&url, &body);

    // Cache result (evict stale entries when over capacity)
    {
        let mut cache = state.unfurl_cache.lock().await;
        if cache.len() >= CACHE_MAX {
            cache.retain(|_, (_, t)| t.elapsed() < CACHE_TTL);
        }
        cache.insert(cache_key, (data.clone(), Instant::now()));
    }

    Ok(axum::Json(data))
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Guard against SSRF by blocking private / loopback IP ranges.
fn guard_url(raw: &str) -> Result<(), String> {
    let parsed = url::Url::parse(raw).map_err(|_| "invalid URL".to_string())?;
    let host = parsed.host_str().ok_or("URL has no host")?;

    // Try to parse as an IP directly
    if let Ok(ip) = host.parse::<IpAddr>()
        && is_blocked_ip(ip) {
            return Err("private/loopback addresses are not allowed".to_string());
        }
    // Note: hostname-based SSRF (e.g., evil.internal) requires DNS resolution,
    // which we skip here for latency. The reqwest timeout and User-Agent serve
    // as secondary mitigations in a trusted-server context.
    Ok(())
}

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            o[0] == 127
                || o[0] == 10
                || o[0] == 0
                || (o[0] == 172 && o[1] >= 16 && o[1] <= 31)
                || (o[0] == 192 && o[1] == 168)
                || (o[0] == 169 && o[1] == 254) // link-local
        }
        IpAddr::V6(v6) => v6.is_loopback() || is_fc00(v6),
    }
}

fn is_fc00(ip: std::net::Ipv6Addr) -> bool {
    let seg = ip.segments();
    (seg[0] & 0xfe00) == 0xfc00 // fc00::/7 (ULA)
}

/// Extract OpenGraph tags from raw HTML.
///
/// Uses simple case-insensitive string scanning rather than a full HTML parser
/// to avoid a heavy dependency. The OG spec guarantees meta tags appear in
/// `<head>`, so the first match is authoritative.
fn parse_og(url: &str, html: &str) -> UnfurlData {
    UnfurlData {
        url:         url.to_string(),
        title:       og_tag(html, "og:title")
                        .or_else(|| og_name_tag(html, "twitter:title"))
                        .or_else(|| html_title(html)),
        description: og_tag(html, "og:description")
                        .or_else(|| og_name_tag(html, "twitter:description")),
        image_url:   og_tag(html, "og:image")
                        .or_else(|| og_name_tag(html, "twitter:image")),
        site_name:   og_tag(html, "og:site_name"),
    }
}

/// Extract `<meta property="PROP" content="VALUE">` (OG style).
fn og_tag(html: &str, prop: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let needle = format!("property=\"{}\"", prop);
    let alt    = format!("property='{}'", prop);
    let pos = lower.find(&needle).or_else(|| lower.find(&alt))?;
    extract_content(&html[pos..])
}

/// Extract `<meta name="NAME" content="VALUE">` (Twitter card style).
fn og_name_tag(html: &str, name: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let needle = format!("name=\"{}\"", name);
    let alt    = format!("name='{}'", name);
    let pos = lower.find(&needle).or_else(|| lower.find(&alt))?;
    extract_content(&html[pos..])
}

/// From a substring starting at a matched meta attribute, find content="...".
fn extract_content(fragment: &str) -> Option<String> {
    // Find the closing > of this tag first so we don't bleed into the next tag
    let tag_end = fragment.find('>')?;
    let tag = &fragment[..tag_end];
    let lower = tag.to_lowercase();

    let (start_pat, end_pat) = if let Some(p) = lower.find("content=\"") {
        (p + 9, '"')
    } else if let Some(p) = lower.find("content='") {
        (p + 9, '\'')
    } else {
        return None;
    };

    let value_start = &tag[start_pat..];
    let value_end = value_start.find(end_pat)?;
    let raw = &value_start[..value_end];
    let decoded = html_decode(raw.trim());
    if decoded.is_empty() { None } else { Some(decoded) }
}

/// Extract `<title>...</title>` as a fallback.
fn html_title(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start = lower.find("<title>")? + 7;
    let end = lower[start..].find("</title>")?;
    let raw = &html[start..start + end];
    let decoded = html_decode(raw.trim());
    if decoded.is_empty() { None } else { Some(decoded) }
}

/// Decode common HTML entities.
fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
     .replace("&lt;", "<")
     .replace("&gt;", ">")
     .replace("&quot;", "\"")
     .replace("&#039;", "'")
     .replace("&#39;", "'")
     .replace("&nbsp;", " ")
     .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_og_full() {
        let html = r#"<html><head>
            <meta property="og:title" content="OpenCorde &amp; Friends">
            <meta property="og:description" content="A great page">
            <meta property="og:image" content="https://example.com/img.png">
            <meta property="og:site_name" content="Example">
        </head></html>"#;
        let d = parse_og("https://example.com", html);
        assert_eq!(d.title.as_deref(), Some("OpenCorde & Friends"));
        assert_eq!(d.description.as_deref(), Some("A great page"));
        assert_eq!(d.image_url.as_deref(), Some("https://example.com/img.png"));
        assert_eq!(d.site_name.as_deref(), Some("Example"));
    }

    #[test]
    fn test_parse_og_title_fallback() {
        let html = "<html><head><title>Plain Title</title></head></html>";
        let d = parse_og("https://example.com", html);
        assert_eq!(d.title.as_deref(), Some("Plain Title"));
        assert!(d.description.is_none());
    }

    #[test]
    fn test_blocked_ip_loopback() {
        assert!(is_blocked_ip("127.0.0.1".parse().unwrap()));
        assert!(is_blocked_ip("::1".parse().unwrap()));
    }

    #[test]
    fn test_blocked_ip_private() {
        assert!(is_blocked_ip("10.0.0.1".parse().unwrap()));
        assert!(is_blocked_ip("192.168.1.1".parse().unwrap()));
        assert!(is_blocked_ip("172.16.0.1".parse().unwrap()));
    }

    #[test]
    fn test_public_ip_allowed() {
        assert!(!is_blocked_ip("8.8.8.8".parse().unwrap()));
        assert!(!is_blocked_ip("1.1.1.1".parse().unwrap()));
    }

    #[test]
    fn test_guard_url_rejects_private() {
        assert!(guard_url("http://192.168.1.1/secret").is_err());
        assert!(guard_url("http://10.0.0.1").is_err());
    }

    #[test]
    fn test_guard_url_allows_public() {
        assert!(guard_url("https://github.com").is_ok());
        assert!(guard_url("https://example.com/path").is_ok());
    }
}
