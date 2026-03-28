//! # Security Headers Middleware
//! Adds HTTP security headers to every API response.
//!
//! ## Headers Applied
//! - `X-Content-Type-Options: nosniff` — Prevents MIME-type sniffing attacks
//! - `X-Frame-Options: DENY` — Prevents clickjacking via iframe embedding
//! - `Referrer-Policy: strict-origin-when-cross-origin` — Limits Referer leakage
//! - `Permissions-Policy` — Disables dangerous browser APIs for API responses
//! - `Content-Security-Policy` — Defence-in-depth for any API-originated HTML
//!
//! ## Note on CSP
//! This CSP applies to API responses. The SvelteKit frontend's CSP is managed
//! separately in its server configuration (svelte.config.js / hooks.server.ts).
//!
//! ## Depends On
//! - axum (middleware, Body, Request, Response)

use axum::{body::Body, http::Request, middleware::Next, response::Response};

/// Axum middleware: inject HTTP security headers into every response.
pub async fn security_headers(req: Request<Body>, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    // Prevent MIME-type sniffing (e.g., serving JSON treated as HTML by browser)
    headers.insert(
        "x-content-type-options",
        "nosniff".parse().expect("static header value"),
    );

    // Block API responses from being embedded in <iframe>
    headers.insert(
        "x-frame-options",
        "DENY".parse().expect("static header value"),
    );

    // Limit Referer header to same-origin for cross-origin requests
    headers.insert(
        "referrer-policy",
        "strict-origin-when-cross-origin"
            .parse()
            .expect("static header value"),
    );

    // Disable dangerous browser features for API-level responses
    headers.insert(
        "permissions-policy",
        "camera=(), microphone=(), geolocation=()"
            .parse()
            .expect("static header value"),
    );

    // Defence-in-depth CSP: allow self + HTTPS for images/media/websockets
    headers.insert(
        "content-security-policy",
        "default-src 'self'; img-src 'self' data: https:; media-src 'self' https:; connect-src 'self' wss: https:"
            .parse()
            .expect("static header value"),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify that the static header values parse without panic.
    // (The real test is that the server starts without panic, but we can verify the values are valid.)
    #[test]
    fn test_header_values_are_valid() {
        "nosniff".parse::<axum::http::HeaderValue>().unwrap();
        "DENY".parse::<axum::http::HeaderValue>().unwrap();
        "strict-origin-when-cross-origin"
            .parse::<axum::http::HeaderValue>()
            .unwrap();
        "camera=(), microphone=(), geolocation=()"
            .parse::<axum::http::HeaderValue>()
            .unwrap();
        "default-src 'self'; img-src 'self' data: https:; media-src 'self' https:; connect-src 'self' wss: https:"
            .parse::<axum::http::HeaderValue>()
            .unwrap();
    }
}
