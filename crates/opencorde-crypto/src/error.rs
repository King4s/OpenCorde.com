//! # Error: CryptoError
//! Unified error type for E2EE cryptographic operations.
//!
//! ## Depends On
//! - thiserror (error deriving)
//! - serde_json (serialization errors)
//! - base64 (encoding/decoding errors)

use thiserror::Error;

/// Errors that can occur during E2EE cryptographic operations.
#[derive(Error, Debug)]
pub enum CryptoError {
    /// Key package generation or validation error
    #[error("key package error: {0}")]
    KeyPackage(String),

    /// MLS group operation error (add/remove members, etc.)
    #[error("group error: {0}")]
    Group(String),

    /// Message encryption error
    #[error("encryption error: {0}")]
    Encryption(String),

    /// Message decryption error
    #[error("decryption error: {0}")]
    Decryption(String),

    /// Serialization/deserialization error
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Base64 decoding error
    #[error("base64 error: {0}")]
    Base64(#[from] base64::DecodeError),
}

impl CryptoError {
    /// Create a KeyPackage error from a string message
    pub fn key_package<S: Into<String>>(msg: S) -> Self {
        Self::KeyPackage(msg.into())
    }

    /// Create a Group error from a string message
    pub fn group<S: Into<String>>(msg: S) -> Self {
        Self::Group(msg.into())
    }

    /// Create an Encryption error from a string message
    pub fn encryption<S: Into<String>>(msg: S) -> Self {
        Self::Encryption(msg.into())
    }

    /// Create a Decryption error from a string message
    pub fn decryption<S: Into<String>>(msg: S) -> Self {
        Self::Decryption(msg.into())
    }
}
