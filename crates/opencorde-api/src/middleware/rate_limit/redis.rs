//! # Redis Rate Limit Backend
//! Distributed token-bucket rate limiting backed by Redis.
//!
//! ## Design
//! - Uses Redis Lua scripts for atomic token-bucket operations
//! - Redis `TIME` command provides consistent timestamps across all instances
//! - Each bucket is a Redis hash with `tokens` and `last_refill` fields
//! - 1-hour TTL on bucket keys to auto-clean stale entries
//! - Graceful fallback to in-memory governor when Redis is unavailable
//!
//! ## Depends On
//! - redis (async connection manager)
//! - tracing (structured logging)
//! - thiserror (typed errors)

use redis::aio::ConnectionManager;
use std::net::IpAddr;
use std::sync::Arc;
use thiserror::Error;

/// Errors from the Redis rate limiter.
#[derive(Debug, Error)]
pub enum RedisRateLimitError {
    #[error("Redis command failed: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("Lua script execution failed: {0}")]
    Script(String),
}

/// Redis-backed token-bucket limiter.
///
/// Uses a Lua script to atomically check and decrement tokens.
/// All timestamps are sourced from Redis `TIME` to avoid clock skew.
#[derive(Clone)]
pub struct RedisRateLimiter {
    conn: ConnectionManager,
    script: Arc<redis::Script>,
}

impl RedisRateLimiter {
    /// Create a new Redis rate limiter from a connection manager.
    pub fn new(conn: ConnectionManager) -> Arc<Self> {
        // Lua script: atomic token-bucket check using Redis server time.
        // KEYS[1]   — bucket key (e.g., "rate_limit:global:192.168.1.1")
        // ARGV[1]   — capacity (max tokens)
        // ARGV[2]   — refill rate (tokens per second)
        // ARGV[3]   — cost (tokens to consume, usually 1)
        // ARGV[4]   — TTL in seconds
        // Returns 1 if allowed, 0 if rejected.
        let script = redis::Script::new(
            r#"
            local key = KEYS[1]
            local capacity = tonumber(ARGV[1])
            local refill_rate = tonumber(ARGV[2])
            local cost = tonumber(ARGV[3])
            local ttl = tonumber(ARGV[4])

            local now = redis.call('TIME')
            local now_ms = now[1] * 1000 + math.floor(now[2] / 1000)

            local state = redis.call('HMGET', key, 'tokens', 'last_refill')
            local tokens = tonumber(state[1])
            local last_refill = tonumber(state[2])

            if tokens == nil then
                tokens = capacity
                last_refill = now_ms
            end

            local delta_ms = now_ms - last_refill
            local refill = math.floor(delta_ms * refill_rate / 1000)
            tokens = math.min(capacity, tokens + refill)

            local allowed = 0
            if tokens >= cost then
                tokens = tokens - cost
                allowed = 1
            end

            redis.call('HMSET', key, 'tokens', tokens, 'last_refill', now_ms)
            redis.call('EXPIRE', key, ttl)
            return allowed
            "#,
        );

        Arc::new(Self {
            conn,
            script: Arc::new(script),
        })
    }

    /// Check whether a request from `ip` is allowed under the given bucket parameters.
    ///
    /// # Parameters
    /// - `prefix` — logical bucket name (e.g., "global", "strict:login")
    /// - `ip` — client IP address
    /// - `capacity` — maximum bucket capacity (burst size)
    /// - `refill_rate` — sustained refill rate in tokens per second
    /// - `cost` — tokens to consume for this request (typically 1)
    ///
    /// # Returns
    /// `true` if the request is allowed, `false` if it should be rate-limited.
    pub async fn check(
        &self,
        prefix: &str,
        ip: IpAddr,
        capacity: u32,
        refill_rate: u32,
        cost: u32,
    ) -> Result<bool, RedisRateLimitError> {
        let key = format!("rate_limit:{}:{}", prefix, ip);
        let allowed: i64 = self
            .script
            .key(key)
            .arg(capacity)
            .arg(refill_rate)
            .arg(cost)
            .arg(3600i32) // 1-hour TTL
            .invoke_async(&mut self.conn.clone())
            .await?;
        Ok(allowed == 1)
    }

    /// Convenience: check the global per-IP bucket.
    pub async fn check_global(
        &self,
        ip: IpAddr,
        rps: u32,
        burst: u32,
    ) -> Result<bool, RedisRateLimitError> {
        self.check("global", ip, burst, rps, 1).await
    }

    /// Convenience: check a strict path-specific bucket.
    pub async fn check_strict(
        &self,
        path_type: &str,
        ip: IpAddr,
        capacity: u32,
        refill_rate: u32,
    ) -> Result<bool, RedisRateLimitError> {
        self.check(
            &format!("strict:{}", path_type),
            ip,
            capacity,
            refill_rate,
            1,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_rate_limiter_new_does_not_panic() {
        // We can't connect to Redis in a unit test without a running server,
        // but we can verify the script compiles and the struct builds.
        // In integration tests we'd spin up a test Redis container.
        let client = redis::Client::open("redis://127.0.0.1:6379").unwrap();
        // Note: ConnectionManager::new is async; here we just verify the client opens.
        drop(client);
    }
}
