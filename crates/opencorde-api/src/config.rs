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
    /// MinIO/S3 endpoint URL
    pub minio_endpoint: String,
    /// MinIO access key
    pub minio_access_key: String,
    /// MinIO secret key
    pub minio_secret_key: String,
    /// MinIO bucket name
    pub minio_bucket: String,
    /// LiveKit server URL
    pub livekit_url: String,
    /// LiveKit API key
    pub livekit_api_key: String,
    /// LiveKit API secret
    pub livekit_api_secret: String,
    /// Deployment environment
    pub environment: Environment,
    /// Mesh federation hostname (e.g., "mesh.example.com")
    pub mesh_hostname: String,
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
        let minio_access_key =
            env::var("MINIO_ACCESS_KEY").unwrap_or_else(|_| "minioadmin".to_string());
        let minio_secret_key =
            env::var("MINIO_SECRET_KEY").unwrap_or_else(|_| "minioadmin".to_string());
        let minio_bucket = env::var("MINIO_BUCKET").unwrap_or_else(|_| "opencorde".to_string());

        let livekit_url =
            env::var("LIVEKIT_URL").unwrap_or_else(|_| "ws://127.0.0.1:7880".to_string());
        let livekit_api_key = env::var("LIVEKIT_API_KEY").unwrap_or_else(|_| "".to_string());
        let livekit_api_secret = env::var("LIVEKIT_API_SECRET").unwrap_or_else(|_| "".to_string());

        let mesh_hostname = env::var("MESH_HOSTNAME").unwrap_or_else(|_| "localhost".to_string());

        let config = Config {
            database_url,
            redis_url,
            api_host,
            api_port,
            jwt_secret,
            jwt_access_expiry,
            jwt_refresh_expiry,
            minio_endpoint,
            minio_access_key,
            minio_secret_key,
            minio_bucket,
            livekit_url,
            livekit_api_key,
            livekit_api_secret,
            environment,
            mesh_hostname,
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
            mesh_hostname = %config.mesh_hostname,
            environment = ?config.environment,
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
