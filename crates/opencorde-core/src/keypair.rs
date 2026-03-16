//! # Ed25519 Keypair Generation
//!
//! Utilities for generating and managing Ed25519 keypairs.
//! Public keys are hex-encoded (64 chars) for storage and transmission.

use ed25519_dalek::SigningKey;
use rand_core::OsRng;

/// Generate a new Ed25519 keypair.
///
/// Returns (private_key_hex, public_key_hex) where both are hex-encoded strings.
///
/// # Returns
/// - Private key (64 hex chars): secret key, store securely in user's device
/// - Public key (64 hex chars): global identity, shared across mesh
#[tracing::instrument]
pub fn generate_keypair() -> (String, String) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let private_key_hex = hex::encode(signing_key.to_bytes());
    let public_key_hex = hex::encode(verifying_key.to_bytes());

    tracing::debug!("Ed25519 keypair generated");

    (private_key_hex, public_key_hex)
}

/// Verify that a public key is valid hex format (64 chars).
pub fn is_valid_public_key(key: &str) -> bool {
    key.len() == 64 && hex::decode(key).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let (private_key, public_key) = generate_keypair();

        // Private key should be 64 hex chars (32 bytes)
        assert_eq!(private_key.len(), 64);
        assert!(hex::decode(&private_key).is_ok());

        // Public key should be 64 hex chars (32 bytes)
        assert_eq!(public_key.len(), 64);
        assert!(hex::decode(&public_key).is_ok());
    }

    #[test]
    fn test_public_key_validation() {
        let (_, public_key) = generate_keypair();
        assert!(is_valid_public_key(&public_key));
    }

    #[test]
    fn test_invalid_public_key() {
        assert!(!is_valid_public_key("tooshort"));
        assert!(!is_valid_public_key(
            "not_hex_chars!@#$%^&*())))))))))))))))))))))))))))))))))))))))"
        ));
        assert!(!is_valid_public_key(&"z".repeat(64))); // Invalid hex (z is not valid hex digit)
    }

    #[test]
    fn test_valid_length_hex() {
        let valid_key = "a".repeat(64); // 64 'a's is valid hex
        assert!(is_valid_public_key(&valid_key));
    }
}
