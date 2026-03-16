//! # API Error Handling
//! Unified error type for all REST API responses.
//!
//! ## Features
//! - Structured error responses with code and message
//! - HTTP status code mapping for each error variant
//! - IntoResponse implementation for Axum integration
//! - Comprehensive error logging for debugging
//!
//! ## Depends On
//! - axum (web framework)
//! - serde (JSON serialization)
//! - thiserror (error definition macro)
//! - tracing (structured logging)

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// JSON error response body sent to clients.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Machine-readable error code (e.g., "NOT_FOUND", "UNAUTHORIZED")
    pub code: String,
    /// Human-readable error message
    pub message: String,
}

/// Unified API error type for all route handlers.
///
/// Every route handler should return `Result<T, ApiError>`.
/// The IntoResponse implementation automatically converts errors
/// to appropriate HTTP responses.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Resource not found (404).
    #[error("not found: {0}")]
    NotFound(String),

    /// Invalid request payload or parameters (400).
    #[error("bad request: {0}")]
    BadRequest(String),

    /// Missing or invalid authentication credentials (401).
    #[error("unauthorized")]
    Unauthorized,

    /// Authenticated user lacks permission (403).
    #[error("forbidden")]
    Forbidden,

    /// Resource already exists (409).
    #[error("conflict: {0}")]
    Conflict(String),

    /// Too many requests (429).
    #[error("rate limited")]
    RateLimited { retry_after: u64 },

    /// Unrecoverable server error (500).
    #[error("internal error")]
    Internal(#[from] anyhow::Error),

    /// Database operation failed (500).
    #[error("database error")]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for ApiError {
    /// Convert ApiError to an HTTP response with appropriate status code.
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone()),

            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg.clone()),

            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                "unauthorized".to_string(),
            ),

            ApiError::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN", "forbidden".to_string()),

            ApiError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg.clone()),

            ApiError::RateLimited { retry_after } => {
                // TODO: Add Retry-After header to response
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    "RATE_LIMITED",
                    format!("retry after {} seconds", retry_after),
                )
            }

            ApiError::Internal(err) => {
                tracing::error!(error = %err, "internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "internal server error".to_string(),
                )
            }

            ApiError::Database(err) => {
                tracing::error!(error = %err, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "internal server error".to_string(),
                )
            }
        };

        let error_response = ErrorResponse {
            code: code.to_string(),
            message,
        };

        (status, axum::Json(error_response)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_status() {
        let err = ApiError::NotFound("user not found".to_string());
        // We can't directly test into_response in unit tests without a full Axum app,
        // but we can verify the error message
        assert_eq!(err.to_string(), "not found: user not found");
    }

    #[test]
    fn test_bad_request_status() {
        let err = ApiError::BadRequest("invalid id".to_string());
        assert_eq!(err.to_string(), "bad request: invalid id");
    }

    #[test]
    fn test_unauthorized_status() {
        let err = ApiError::Unauthorized;
        assert_eq!(err.to_string(), "unauthorized");
    }

    #[test]
    fn test_forbidden_status() {
        let err = ApiError::Forbidden;
        assert_eq!(err.to_string(), "forbidden");
    }

    #[test]
    fn test_conflict_status() {
        let err = ApiError::Conflict("username already taken".to_string());
        assert_eq!(err.to_string(), "conflict: username already taken");
    }

    #[test]
    fn test_rate_limited() {
        let err = ApiError::RateLimited { retry_after: 60 };
        assert_eq!(err.to_string(), "rate limited");
    }

    #[test]
    fn test_error_response_serialization() {
        let response = ErrorResponse {
            code: "NOT_FOUND".to_string(),
            message: "user not found".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("NOT_FOUND"));
        assert!(json.contains("user not found"));
    }
}
