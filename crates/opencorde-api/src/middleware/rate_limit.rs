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
    http::{HeaderMap, Method, Request, StatusCode},
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
    fn limiter_for(
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
        } else if path.starts_with("/api/v1/auth/forgot-password")
            || path.starts_with("/api/v1/auth/reset-password")
        {
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
    /// Immutable transport peer trusted to overwrite X-Real-IP.
    trusted_proxy_ip: Option<IpAddr>,
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
        Self::with_trusted_proxy(config, None)
    }

    pub fn with_trusted_proxy(
        config: RateLimitConfig,
        trusted_proxy_ip: Option<IpAddr>,
    ) -> Arc<Self> {
        let limiter = Arc::new(build_limiter(config.requests_per_second, config.burst_size));
        Arc::new(Self {
            trusted_proxy_ip: trusted_proxy_ip.map(|ip| ip.to_canonical()),
            config: RwLock::new(config),
            limiter: RwLock::new(limiter),
            strict: StrictPathLimits::new(),
        })
    }

    fn client_ip(&self, peer: IpAddr, headers: &HeaderMap) -> IpAddr {
        let peer = peer.to_canonical();
        if Some(peer) != self.trusted_proxy_ip {
            return peer;
        }
        // The trusted proxy supplies exactly one bare address. Ambiguous or
        // malformed input shares the socket peer's bucket.
        let real_ip = headers.get_all("x-real-ip");
        let mut values = real_ip.iter();
        let (Some(value), None) = (values.next(), values.next()) else {
            return peer;
        };
        value
            .to_str()
            .ok()
            .and_then(|text| text.parse::<IpAddr>().ok())
            .map(|ip| ip.to_canonical())
            .unwrap_or(peer)
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
    let ip = rl.client_ip(addr.ip(), req.headers());
    let path = req.uri().path().to_owned();
    let method = req.method().clone();

    // Strict path-specific check (always runs)
    if let Some(strict_limiter) = rl.strict.limiter_for(&method, &path)
        && strict_limiter.check_key(&ip).is_err()
    {
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
    use axum::{Router, middleware::from_fn_with_state, routing::post};
    use tower::ServiceExt;

    fn proxy_test_config() -> RateLimitConfig {
        RateLimitConfig {
            requests_per_second: 100,
            burst_size: 200,
            enabled: true,
        }
    }

    #[tokio::test]
    async fn login_quota_is_independent_for_clients_behind_the_trusted_proxy() {
        let proxy: SocketAddr = "192.0.2.10:40000".parse().unwrap();
        let state = RateLimitState::with_trusted_proxy(proxy_test_config(), Some(proxy.ip()));
        let app = Router::new()
            .route("/api/v1/auth/login", post(|| async { StatusCode::OK }))
            .layer(from_fn_with_state(state, rate_limit_middleware));
        let login = |client: &'static str| {
            let app = app.clone();
            async move {
                let request = Request::post("/api/v1/auth/login")
                    .extension(ConnectInfo(proxy))
                    .header("x-real-ip", client)
                    .header("x-forwarded-for", "203.0.113.99")
                    .body(Body::empty())
                    .unwrap();
                app.oneshot(request).await.unwrap().status()
            }
        };
        for _ in 0..5 {
            assert_eq!(login("198.51.100.1").await, StatusCode::OK);
        }
        assert_eq!(
            login("::ffff:198.51.100.1").await,
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(login("198.51.100.2").await, StatusCode::OK);
    }

    #[test]
    fn untrusted_or_ambiguous_forwarded_addresses_use_the_socket_peer() {
        let proxy: IpAddr = "192.0.2.10".parse().unwrap();
        let direct: IpAddr = "192.0.2.11".parse().unwrap();
        let state = RateLimitState::with_trusted_proxy(proxy_test_config(), Some(proxy));
        let standalone = RateLimitState::new(proxy_test_config());
        let mut headers = HeaderMap::new();
        headers.insert("forwarded", "for=203.0.113.1".parse().unwrap());
        headers.insert("x-forwarded-for", "203.0.113.1".parse().unwrap());
        assert_eq!(state.client_ip(proxy, &headers), proxy);

        headers.insert("x-real-ip", "203.0.113.1".parse().unwrap());
        assert_eq!(state.client_ip(direct, &headers), direct);
        assert_eq!(standalone.client_ip(proxy, &headers), proxy);
        headers.append("x-real-ip", "203.0.113.2".parse().unwrap());
        assert_eq!(state.client_ip(proxy, &headers), proxy);
        for malformed in [
            "",
            "not-an-ip",
            "203.0.113.1:80",
            "203.0.113.1, 203.0.113.2",
        ] {
            headers.insert("x-real-ip", malformed.parse().unwrap());
            assert_eq!(state.client_ip(proxy, &headers), proxy);
        }
    }

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
        // Password reset (forgot-password)
        assert!(
            limits
                .limiter_for(&Method::POST, "/api/v1/auth/forgot-password")
                .is_some()
        );
        // Password reset (reset-password)
        assert!(
            limits
                .limiter_for(&Method::POST, "/api/v1/auth/reset-password")
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
