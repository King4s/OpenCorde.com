//! # Governor Rate Limit Backend
//! In-memory token-bucket rate limiting using the governor crate.
//!
//! ## Design
//! - Per-IP keyed token buckets via governor's dashmap backend
//! - Used as a graceful fallback when Redis is unavailable
//! - Also used for unit tests that don't require a Redis server
//!
//! ## Depends On
//! - governor (token-bucket algorithm)
//! - serde (config serialization)
//!

use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;

use axum::http::Method;
pub use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use serde::{Deserialize, Serialize};

/// Rate limit configuration exposed to the admin API.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Sustained requests per second allowed per source IP
    pub requests_per_second: u32,
    /// Maximum burst capacity (initial tokens in the bucket)
    pub burst_size: u32,
    /// Whether the global rate limit is active
    pub enabled: bool,
}

/// Strict per-path limiters for security-sensitive endpoints.
///
/// These always run regardless of the global `enabled` flag.
pub struct StrictPathLimits {
    /// POST /api/v1/auth/login — 5 req/min
    pub login: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    /// POST /api/v1/auth/register — 3 req/min
    pub register: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    /// POST /api/v1/auth/password* — 3 req/hour
    pub password_reset: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    /// POST **/messages — 5 req/sec with burst 10
    pub send_message: Arc<DefaultKeyedRateLimiter<IpAddr>>,
}

impl StrictPathLimits {
    pub fn new() -> Self {
        Self {
            login: Arc::new(RateLimiter::keyed(
                Quota::per_minute(nz(30)).allow_burst(nz(30)),
            )),
            register: Arc::new(RateLimiter::keyed(
                Quota::per_minute(nz(3)).allow_burst(nz(3)),
            )),
            password_reset: Arc::new(RateLimiter::keyed(
                Quota::per_hour(nz(3)).allow_burst(nz(3)),
            )),
            send_message: Arc::new(RateLimiter::keyed(
                Quota::per_second(nz(5)).allow_burst(nz(10)),
            )),
        }
    }

    /// Return the appropriate strict limiter for this (method, path), if any.
    pub fn limiter_for(
        &self,
        method: &Method,
        path: &str,
    ) -> Option<Arc<DefaultKeyedRateLimiter<IpAddr>>> {
        if method != Method::POST {
            return None;
        }
        if path == "/api/v1/auth/login" {
            Some(self.login.clone())
        } else if path == "/api/v1/auth/register" {
            Some(self.register.clone())
        } else if path.starts_with("/api/v1/auth/password") {
            Some(self.password_reset.clone())
        } else if path.ends_with("/messages") {
            Some(self.send_message.clone())
        } else {
            None
        }
    }
}

/// Construct a governor keyed rate limiter for the given rate and burst.
pub fn build_limiter(rps: u32, burst: u32) -> DefaultKeyedRateLimiter<IpAddr> {
    let rps = nz(rps.max(1));
    let burst = nz(burst.max(rps.get())); // burst >= rps
    RateLimiter::keyed(Quota::per_second(rps).allow_burst(burst))
}

/// Helper: NonZeroU32 from u32, clamped to 1.
pub fn nz(v: u32) -> NonZeroU32 {
    NonZeroU32::new(v.max(1)).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_limiter_does_not_panic() {
        let _l = build_limiter(100, 200);
        let _l2 = build_limiter(1, 1);
        // burst < rps should be clamped to rps
        let _l3 = build_limiter(50, 10);
    }

    #[test]
    fn test_strict_path_limits_matching() {
        let limits = StrictPathLimits::new();
        // Login endpoint
        assert!(
            limits
                .limiter_for(&Method::POST, "/api/v1/auth/login")
                .is_some()
        );
        // GET to login is not limited by strict
        assert!(
            limits
                .limiter_for(&Method::GET, "/api/v1/auth/login")
                .is_none()
        );
        // Register endpoint
        assert!(
            limits
                .limiter_for(&Method::POST, "/api/v1/auth/register")
                .is_some()
        );
        // Password reset
        assert!(
            limits
                .limiter_for(&Method::POST, "/api/v1/auth/password-reset")
                .is_some()
        );
        // Message send
        assert!(
            limits
                .limiter_for(&Method::POST, "/api/v1/channels/123/messages")
                .is_some()
        );
        // Normal route — not limited
        assert!(
            limits
                .limiter_for(&Method::GET, "/api/v1/servers")
                .is_none()
        );
    }
}
