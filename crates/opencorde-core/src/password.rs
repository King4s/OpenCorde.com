//! # Password Hashing
//! Argon2id password hashing and verification (OWASP recommended).
//!
//! Provides secure password hashing with automatic random salt generation
//! and verification against stored hashes.
//!
//! ## Security Properties
//! - Argon2id algorithm with sensible defaults
//! - Random salt per hash (prevents rainbow table attacks)
//! - Memory-hard hashing (resists GPU/ASIC attacks)
//!
//! ## Features
//! - `hash_password` — Hash a plain-text password for storage
//! - `verify_password` — Verify a password against a stored hash
//!
//! ## Depends On
//! - argon2 crate (workspace dependency)

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use rand_core::OsRng;
use tracing::instrument;

/// Hash a password using Argon2id with a random salt.
///
/// # Arguments
/// * `password` — The plain-text password to hash
///
/// # Returns
/// The password hash string suitable for storage in a database.
///
/// # Errors
/// Returns `argon2::password_hash::Error` if hashing fails.
#[instrument(skip(password))]
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    tracing::debug!("password hashed successfully");
    Ok(hash.to_string())
}

/// Verify a password against a stored hash.
///
/// # Arguments
/// * `password` — The plain-text password to verify
/// * `hash` — The stored password hash
///
/// # Returns
/// - `Ok(true)` if the password matches the hash
/// - `Ok(false)` if the password does not match
/// - `Err(e)` if parsing or verification fails
///
/// # Errors
/// Returns `argon2::password_hash::Error` if parsing the hash fails.
#[instrument(skip(password, hash))]
pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(hash)?;
    let argon2 = Argon2::default();
    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(()) => {
            tracing::debug!("password verified successfully");
            Ok(true)
        }
        Err(argon2::password_hash::Error::Password) => {
            tracing::debug!("password verification failed: incorrect password");
            Ok(false)
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password_success() {
        let password = "my_secure_password_123";
        let hash = hash_password(password).expect("hash should succeed");
        assert!(!hash.is_empty());
        assert!(hash.contains("$argon2"));
    }

    #[test]
    fn test_verify_correct_password() {
        let password = "correct_password";
        let hash = hash_password(password).expect("hash should succeed");
        let result = verify_password(password, &hash).expect("verify should succeed");
        assert!(result, "correct password should verify successfully");
    }

    #[test]
    fn test_verify_wrong_password() {
        let password = "correct_password";
        let wrong_password = "wrong_password";
        let hash = hash_password(password).expect("hash should succeed");
        let result = verify_password(wrong_password, &hash).expect("verify should succeed");
        assert!(!result, "wrong password should not verify");
    }

    #[test]
    fn test_different_hashes_same_password() {
        let password = "same_password";
        let hash1 = hash_password(password).expect("hash1 should succeed");
        let hash2 = hash_password(password).expect("hash2 should succeed");
        // Two hashes of the same password should be different (random salt)
        assert_ne!(hash1, hash2, "hashes with random salts should differ");
        // But both should verify against the original password
        assert!(
            verify_password(password, &hash1).expect("verify hash1 should succeed"),
            "hash1 should verify"
        );
        assert!(
            verify_password(password, &hash2).expect("verify hash2 should succeed"),
            "hash2 should verify"
        );
    }

    #[test]
    fn test_empty_password_hashing() {
        let password = "";
        let hash = hash_password(password).expect("empty password should hash");
        let result = verify_password(password, &hash).expect("verify should succeed");
        assert!(result, "empty password should verify successfully");
    }

    #[test]
    fn test_long_password() {
        let password = "a".repeat(1000);
        let hash = hash_password(&password).expect("long password should hash");
        let result = verify_password(&password, &hash).expect("verify should succeed");
        assert!(result, "long password should verify successfully");
    }

    #[test]
    fn test_special_characters_in_password() {
        let password = "P@$$w0rd!#%&*()[]{}|;:',.<>?/~`";
        let hash = hash_password(password).expect("special char password should hash");
        let result = verify_password(password, &hash).expect("verify should succeed");
        assert!(result, "special character password should verify");
    }
}
