//! # Middleware: CORS
//! Cross-Origin Resource Sharing configuration for REST API.
//!
//! ## Features
//! - Environment-aware CORS policies (strict in production, permissive in dev)
//! - Configurable allowed origins, methods, and headers
//! - Credentials support for cookie-based auth
//!
//! ## Depends On
//! - tower_http (CORS layer implementation)
//! - axum (HTTP primitives)

use axum::http::{HeaderName, Method};
use tower_http::cors::CorsLayer;

/// Create a CORS middleware layer configured for the environment.
///
/// In production: Strict whitelist of allowed origins.
/// In development: Permissive CORS (allow all origins).
///
/// # Arguments
/// * `is_production` - Whether the server is running in production mode
pub fn cors_layer(is_production: bool) -> CorsLayer {
    if is_production {
        // Production: strict CORS policy
        // Allow specific origin for the OpenCorde domain
        CorsLayer::new()
            .allow_origin(axum::http::HeaderValue::from_static(
                "https://opencorde.com",
            ))
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                HeaderName::from_static("content-type"),
                HeaderName::from_static("authorization"),
                HeaderName::from_static("x-request-id"),
            ])
            .allow_credentials(true)
            .max_age(std::time::Duration::from_secs(3600))
    } else {
        // Development: permissive CORS (allow all origins)
        CorsLayer::permissive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_layer_creation_production() {
        let _cors = cors_layer(true);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_cors_layer_creation_development() {
        let _cors = cors_layer(false);
        // Just verify it doesn't panic
    }
}
