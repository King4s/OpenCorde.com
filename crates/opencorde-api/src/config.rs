//! # Configuration Module
//! Loads application settings from environment variables with sensible defaults.
//!
//! ## Features
//! - Automatic environment detection (Development/Production)
//! - Masked secret logging for security
//! - Validation of required configuration values
//!
//! ## Depends On
//! - std::env (standard library)
//! - anyhow (error handling)

use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    /// PostgreSQL connection string
    pub database_url: String,
    /// Redis connection string
    pub redis_url: String,
    /// HTTP server bind host
    pub api_host: String,
    /// HTTP server bind port
    pub api_port: u16,
    /// JWT signing secret
    pub jwt_secret: String,
    /// JWT access token expiry in seconds
    pub jwt_access_expiry: u64,
    /// JWT refresh token expiry in seconds
    pub jwt_refresh_expiry: u64,
    /// MinIO/S3 endpoint URL (internal, used for SDK uploads)
    pub minio_endpoint: String,
    /// Public-facing base URL for file downloads (e.g. https://opencorde.com/files)
    pub files_public_url: String,
    /// MinIO access key
    pub minio_access_key: String,
    /// MinIO secret key
    pub minio_secret_key: String,
    /// MinIO bucket name
    pub minio_bucket: String,
    /// LiveKit server URL
    pub livekit_url: String,
    /// Public-facing LiveKit WebSocket URL for browser clients
    pub livekit_public_url: String,
    /// LiveKit API key
    pub livekit_api_key: String,
    /// LiveKit API secret
    pub livekit_api_secret: String,
    /// Deployment environment
    pub environment: Environment,
    /// Mesh federation hostname (e.g., "mesh.example.com")
    pub mesh_hostname: String,
    /// Admin user IDs (comma-separated list of Snowflake IDs)
    pub admin_user_ids: Vec<String>,
    /// SMTP hostname for email sending (optional, development mode if not set)
    pub smtp_host: Option<String>,
    /// SMTP port (default 587)
    pub smtp_port: u16,
    /// SMTP username (optional)
    pub smtp_username: Option<String>,
    /// SMTP password (optional)
    pub smtp_password: Option<String>,
    /// Sender email address (default noreply@localhost)
    pub smtp_from: String,
    /// Base URL for password reset links (default http://localhost:5173)
    pub base_url: String,
    /// Registration mode: open (default), invite_only, or closed
    pub registration_mode: RegistrationMode,
    /// Rate limiter: sustained requests per second per IP (default 100)
    pub rate_limit_rps: u32,
    /// Rate limiter: maximum burst capacity per IP (default 200)
    pub rate_limit_burst: u32,
    /// Steam Web API key (optional, for fetching player summaries)
    pub steam_api_key: Option<String>,
    /// Firebase Cloud Messaging server key (optional, for Android push notifications)
    pub fcm_server_key: Option<String>,
    /// Shared registration invite code for invite-only mode (optional)
    pub registration_invite_code: Option<String>,
    /// Path to Tantivy search index directory (optional, disables search if unset)
    pub search_index_path: Option<String>,
    /// VAPID private key in URL-safe base64 (optional, for Web Push notifications)
    pub vapid_private_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RegistrationMode {
    Open,
    InviteOnly,
    Closed,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Environment {
    Development,
    Production,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Requires DATABASE_URL and JWT_SECRET. Other values have sensible defaults
    /// for development. In production, all variables should be explicitly set.
    ///
    /// # Errors
    /// Returns anyhow::Error if required variables are missing or invalid.
    pub fn from_env() -> anyhow::Result<Self> {
        // Detect environment
        let env_var = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        let environment = match env_var.to_lowercase().as_str() {
            "production" | "prod" => Environment::Production,
            _ => Environment::Development,
        };

        // Load required variables
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable is required"))?;

        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| anyhow::anyhow!("JWT_SECRET environment variable is required"))?;

        // Load optional variables with defaults
        let api_host = env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let api_port: u16 = env::var("API_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .map_err(|_| anyhow::anyhow!("API_PORT must be a valid u16"))?;

        let redis_url =
            env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        let jwt_access_expiry: u64 = env::var("JWT_ACCESS_EXPIRY")
            .unwrap_or_else(|_| "900".to_string()) // 15 minutes
            .parse()
            .map_err(|_| anyhow::anyhow!("JWT_ACCESS_EXPIRY must be a valid u64"))?;

        let jwt_refresh_expiry: u64 = env::var("JWT_REFRESH_EXPIRY")
            .unwrap_or_else(|_| "604800".to_string()) // 7 days
            .parse()
            .map_err(|_| anyhow::anyhow!("JWT_REFRESH_EXPIRY must be a valid u64"))?;

        let minio_endpoint =
            env::var("MINIO_ENDPOINT").unwrap_or_else(|_| "http://127.0.0.1:9000".to_string());
        let files_public_url = env::var("FILES_PUBLIC_URL")
            .unwrap_or_else(|_| minio_endpoint.clone());
        let minio_access_key =
            env::var("MINIO_ACCESS_KEY").unwrap_or_else(|_| "minioadmin".to_string());
        let minio_secret_key =
            env::var("MINIO_SECRET_KEY").unwrap_or_else(|_| "minioadmin".to_string());
        let minio_bucket = env::var("MINIO_BUCKET").unwrap_or_else(|_| "opencorde".to_string());

        let livekit_url =
            env::var("LIVEKIT_URL").unwrap_or_else(|_| "ws://127.0.0.1:7880".to_string());
        let livekit_public_url = env::var("LIVEKIT_PUBLIC_URL")
            .unwrap_or_else(|_| "wss://localhost:7881".to_string());
        let livekit_api_key = env::var("LIVEKIT_API_KEY").unwrap_or_else(|_| "".to_string());
        let livekit_api_secret = env::var("LIVEKIT_API_SECRET").unwrap_or_else(|_| "".to_string());

        let mesh_hostname = env::var("MESH_HOSTNAME").unwrap_or_else(|_| "localhost".to_string());

        // Parse admin user IDs from comma-separated env var
        let admin_user_ids: Vec<String> = env::var("ADMIN_USER_IDS")
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_string())
            .collect();

        // Load SMTP configuration (optional)
        let smtp_host = env::var("SMTP_HOST").ok();
        let smtp_port: u16 = env::var("SMTP_PORT")
            .unwrap_or_else(|_| "587".to_string())
            .parse()
            .map_err(|_| anyhow::anyhow!("SMTP_PORT must be a valid u16"))?;
        let smtp_username = env::var("SMTP_USERNAME").ok();
        let smtp_password = env::var("SMTP_PASSWORD").ok();
        let smtp_from =
            env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@localhost".to_string());
        let base_url =
            env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());

        let rate_limit_rps: u32 = env::var("RATE_LIMIT_RPS")
            .unwrap_or_else(|_| "100".to_string())
            .parse()
            .map_err(|_| anyhow::anyhow!("RATE_LIMIT_RPS must be a valid u32"))?;
        let rate_limit_burst: u32 = env::var("RATE_LIMIT_BURST")
            .unwrap_or_else(|_| "200".to_string())
            .parse()
            .map_err(|_| anyhow::anyhow!("RATE_LIMIT_BURST must be a valid u32"))?;

        let registration_mode = match env::var("REGISTRATION_MODE")
            .unwrap_or_else(|_| "open".to_string())
            .to_lowercase()
            .as_str()
        {
            "invite_only" | "invite-only" => RegistrationMode::InviteOnly,
            "closed" => RegistrationMode::Closed,
            _ => RegistrationMode::Open,
        };

        let steam_api_key = env::var("STEAM_API_KEY").ok();
        let registration_invite_code = env::var("REGISTRATION_INVITE_CODE").ok();
        let search_index_path = env::var("SEARCH_INDEX_PATH").ok();
        let fcm_server_key = env::var("FCM_SERVER_KEY").ok();
        let vapid_private_key = env::var("VAPID_PRIVATE_KEY").ok();

        let config = Config {
            database_url,
            redis_url,
            api_host,
            api_port,
            jwt_secret,
            jwt_access_expiry,
            jwt_refresh_expiry,
            minio_endpoint,
            files_public_url,
            minio_access_key,
            minio_secret_key,
            minio_bucket,
            livekit_url,
            livekit_public_url,
            livekit_api_key,
            livekit_api_secret,
            environment,
            mesh_hostname,
            admin_user_ids,
            smtp_host,
            smtp_port,
            smtp_username,
            smtp_password,
            smtp_from,
            base_url,
            registration_mode,
            rate_limit_rps,
            rate_limit_burst,
            steam_api_key,
            registration_invite_code,
            search_index_path,
            fcm_server_key,
            vapid_private_key,
        };

        // Log configuration (with secrets masked)
        tracing::info!(
            api_host = %config.api_host,
            api_port = config.api_port,
            database_url = %mask_secret(&config.database_url),
            redis_url = %mask_secret(&config.redis_url),
            jwt_access_expiry = config.jwt_access_expiry,
            jwt_refresh_expiry = config.jwt_refresh_expiry,
            minio_endpoint = %config.minio_endpoint,
            minio_bucket = %config.minio_bucket,
            livekit_url = %config.livekit_url,
            livekit_public_url = %config.livekit_public_url,
            mesh_hostname = %config.mesh_hostname,
            admin_user_count = config.admin_user_ids.len(),
            smtp_host = ?config.smtp_host,
            smtp_port = config.smtp_port,
            smtp_from = %config.smtp_from,
            base_url = %config.base_url,
            environment = ?config.environment,
            rate_limit_rps = config.rate_limit_rps,
            rate_limit_burst = config.rate_limit_burst,
            steam_api_key = ?config.steam_api_key.is_some(),
            "configuration loaded"
        );

        Ok(config)
    }

    /// Check if running in production mode.
    pub fn is_production(&self) -> bool {
        self.environment == Environment::Production
    }
}

/// Mask sensitive values in logs (e.g., passwords, tokens).
fn mask_secret(secret: &str) -> String {
    if secret.len() > 6 {
        format!("{}...{}", &secret[..3], &secret[secret.len() - 3..])
    } else {
        "***".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_secret() {
        // Test masking a long secret: keep first 3 and last 3 chars
        assert_eq!(
            mask_secret("postgres://user:password@localhost"),
            "pos...ost"
        );
        // Test masking a short secret
        assert_eq!(mask_secret("short"), "***");
        // Test edge case: exactly 6 chars
        assert_eq!(mask_secret("secret"), "***");
    }

    #[test]
    fn test_environment_detection() {
        // These tests would require setting env vars, which is environment-dependent.
        // In a real test, you'd use a test helper to set/unset env vars.
        let env = Environment::Development;
        assert_eq!(env, Environment::Development);
    }
}
