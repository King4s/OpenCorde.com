//! # Bridge Configuration
//! Loads bridge settings from environment variables.
//!
//! ## Required
//! - DISCORD_TOKEN — Discord bot token
//! - DATABASE_URL  — PostgreSQL connection string
//!
//! ## Depends On
//! - std::env

use std::env;

/// Configuration for the OpenCorde ↔ Discord bridge service.
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    /// Discord bot token (from Discord Developer Portal)
    pub discord_token: String,
    /// PostgreSQL connection string
    pub database_url: String,
    /// How often to poll for new OpenCorde messages to forward to Discord (ms)
    pub poll_interval_ms: u64,
}

impl BridgeConfig {
    /// Load configuration from environment variables.
    ///
    /// # Errors
    /// Returns an error if DISCORD_TOKEN or DATABASE_URL are not set.
    pub fn from_env() -> anyhow::Result<Self> {
        let discord_token = env::var("DISCORD_TOKEN")
            .map_err(|_| anyhow::anyhow!("DISCORD_TOKEN is required"))?;

        let database_url = env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL is required"))?;

        let poll_interval_ms: u64 = env::var("BRIDGE_POLL_INTERVAL_MS")
            .unwrap_or_else(|_| "2000".to_string())
            .parse()
            .map_err(|_| anyhow::anyhow!("BRIDGE_POLL_INTERVAL_MS must be a valid u64"))?;

        tracing::info!(
            poll_interval_ms,
            "bridge configuration loaded"
        );

        Ok(Self {
            discord_token,
            database_url,
            poll_interval_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_missing_token() {
        // Without env vars set, from_env should error
        std::env::remove_var("DISCORD_TOKEN");
        std::env::remove_var("DATABASE_URL");
        assert!(BridgeConfig::from_env().is_err());
    }
}
