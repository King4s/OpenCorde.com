//! # Rate Limit Middleware
//! Per-IP request rate limiting using the governor crate.
//!
//! ## Design
//! - Global per-IP token-bucket (configurable via admin API)
//! - Strict per-path limiters for sensitive endpoints (hardcoded):
//!   - POST /api/v1/auth/login      — 5/min (brute-force protection)
//!   - POST /api/v1/auth/register   — 3/min (account creation)
//!   - POST /api/v1/auth/password   — 3/hour (email flooding)
//!   - POST .../messages            — 5/sec sustained (spam)
//! - Requests over any limit get HTTP 429 Too Many Requests
//! - Global limit disabled when `enabled = false`; strict limits always active
//!
//! ## Depends On
//! - governor (token-bucket algorithm, dashmap feature for keyed limiter)
//! - axum (middleware, ConnectInfo extractor)
//! - tokio (async RwLock)

use std::net::{IpAddr, SocketAddr};
use std::num::NonZeroU32;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{Method, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

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
struct StrictPathLimits {
    /// POST /api/v1/auth/login — 5 req/min
    login: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    /// POST /api/v1/auth/register — 3 req/min
    register: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    /// POST /api/v1/auth/password* — 3 req/hour
    password_reset: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    /// POST **/messages — 5 req/sec with burst 10
    send_message: Arc<DefaultKeyedRateLimiter<IpAddr>>,
}

impl StrictPathLimits {
    fn new() -> Self {
        Self {
            login: Arc::new(RateLimiter::keyed(
                Quota::per_minute(nz(5)).allow_burst(nz(5)),
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
    fn limiter_for(&self, method: &Method, path: &str) -> Option<Arc<DefaultKeyedRateLimiter<IpAddr>>> {
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

/// Shared rate limiter state stored in AppState.
pub struct RateLimitState {
    /// Current global configuration (admin-writeable)
    pub config: RwLock<RateLimitConfig>,
    /// Active global limiter — replaced atomically when config changes
    limiter: RwLock<Arc<DefaultKeyedRateLimiter<IpAddr>>>,
    /// Strict per-path limiters (fixed, not admin-configurable)
    strict: StrictPathLimits,
}

impl RateLimitState {
    /// Create a new RateLimitState from an initial config.
    pub fn new(config: RateLimitConfig) -> Arc<Self> {
        let limiter = Arc::new(build_limiter(config.requests_per_second, config.burst_size));
        Arc::new(Self {
            config: RwLock::new(config),
            limiter: RwLock::new(limiter),
            strict: StrictPathLimits::new(),
        })
    }

    /// Rebuild the global governor limiter from the current config.
    ///
    /// Call this after writing new values to `self.config`.
    pub async fn rebuild_limiter(&self) {
        let cfg = self.config.read().await;
        let fresh = Arc::new(build_limiter(cfg.requests_per_second, cfg.burst_size));
        drop(cfg);
        *self.limiter.write().await = fresh;
    }
}

/// Construct a governor keyed rate limiter for the given rate and burst.
fn build_limiter(rps: u32, burst: u32) -> DefaultKeyedRateLimiter<IpAddr> {
    let rps = nz(rps.max(1));
    let burst = nz(burst.max(rps.get())); // burst >= rps
    RateLimiter::keyed(Quota::per_second(rps).allow_burst(burst))
}

/// Helper: NonZeroU32 from u32, clamped to 1.
fn nz(v: u32) -> NonZeroU32 {
    NonZeroU32::new(v.max(1)).unwrap()
}

/// Axum middleware: enforce per-IP rate limits before passing to handler.
///
/// 1. Checks strict path-specific limit (always active).
/// 2. Checks global per-IP limit (when `enabled = true`).
///
/// Returns HTTP 429 if either limit is exceeded.
pub async fn rate_limit_middleware(
    State(rl): State<Arc<RateLimitState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let ip = addr.ip();
    let path = req.uri().path().to_owned();
    let method = req.method().clone();

    // Strict path-specific check (always runs)
    if let Some(strict_limiter) = rl.strict.limiter_for(&method, &path)
        && strict_limiter.check_key(&ip).is_err() {
            tracing::warn!(
                client_ip = %ip,
                path = %path,
                "strict path rate limit exceeded"
            );
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }

    // Global per-IP check
    let enabled = rl.config.read().await.enabled;
    if enabled {
        let limiter = rl.limiter.read().await.clone();
        if limiter.check_key(&ip).is_err() {
            tracing::warn!(client_ip = %ip, "global rate limit exceeded");
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
    }

    Ok(next.run(req).await)
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
        assert!(limits.limiter_for(&Method::POST, "/api/v1/auth/login").is_some());
        // GET to login is not limited by strict
        assert!(limits.limiter_for(&Method::GET, "/api/v1/auth/login").is_none());
        // Register endpoint
        assert!(limits.limiter_for(&Method::POST, "/api/v1/auth/register").is_some());
        // Password reset
        assert!(limits.limiter_for(&Method::POST, "/api/v1/auth/password-reset").is_some());
        // Message send
        assert!(limits.limiter_for(&Method::POST, "/api/v1/channels/123/messages").is_some());
        // Normal route — not limited
        assert!(limits.limiter_for(&Method::GET, "/api/v1/servers").is_none());
    }

    #[tokio::test]
    async fn test_rate_limit_state_new() {
        let cfg = RateLimitConfig {
            requests_per_second: 100,
            burst_size: 200,
            enabled: true,
        };
        let state = RateLimitState::new(cfg);
        let config = state.config.read().await;
        assert_eq!(config.requests_per_second, 100);
        assert_eq!(config.burst_size, 200);
        assert!(config.enabled);
    }

    #[tokio::test]
    async fn test_rebuild_limiter() {
        let cfg = RateLimitConfig {
            requests_per_second: 10,
            burst_size: 20,
            enabled: true,
        };
        let state = RateLimitState::new(cfg);
        {
            let mut config = state.config.write().await;
            config.requests_per_second = 50;
            config.burst_size = 100;
        }
        state.rebuild_limiter().await; // should not panic
    }
}
