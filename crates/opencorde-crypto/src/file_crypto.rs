//! # AES-256-GCM File Encryption
//!
//! Encrypt and decrypt raw file bytes for E2EE attachment uploads.
//!
//! ## Wire Format
//! ```text
//! [12-byte random IV] ++ [AES-256-GCM ciphertext + 16-byte GCM tag]
//! ```
//! The GCM tag is appended to the ciphertext automatically by `aes-gcm`.
//! Total overhead: 28 bytes per file.
//!
//! ## Key
//! 32-byte key derived from the channel's MLS group via `group::export_file_key()`.
//! The key rotates on every epoch transition (member add/remove) giving
//! automatic forward secrecy for new uploads.
//!
//! ## Depends On
//! - aes-gcm 0.10 (AES-256-GCM AEAD cipher)
//! - rand 0.8 (IV generation)
//! - crate::error (CryptoError)

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use rand::RngCore;

use crate::error::CryptoError;

const IV_LEN: usize = 12; // GCM standard nonce length

/// Encrypt `plaintext` bytes with a 32-byte AES-256-GCM key.
///
/// Returns `IV (12 bytes) || ciphertext` as a single `Vec<u8>`.
/// A fresh random IV is generated for every call.
pub fn encrypt_bytes(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

    let mut iv = [0u8; IV_LEN];
    rand::thread_rng().fill_bytes(&mut iv);
    let nonce = Nonce::from_slice(&iv);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CryptoError::encryption(format!("AES-GCM encrypt failed: {e}")))?;

    let mut out = Vec::with_capacity(IV_LEN + ciphertext.len());
    out.extend_from_slice(&iv);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypt an encrypted blob `IV (12 bytes) || ciphertext` with a 32-byte AES-256-GCM key.
///
/// Returns the plaintext bytes. Fails if the GCM tag is invalid (tampered data).
pub fn decrypt_bytes(key: &[u8; 32], encrypted: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if encrypted.len() < IV_LEN + 16 {
        // 16 = minimum GCM tag length
        return Err(CryptoError::decryption(
            "encrypted blob too short (expected IV + ciphertext)"
        ));
    }

    let (iv_bytes, ciphertext) = encrypted.split_at(IV_LEN);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(iv_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CryptoError::decryption(format!("AES-GCM decrypt failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        [0x42u8; 32]
    }

    #[test]
    fn test_roundtrip() {
        let key = test_key();
        let plaintext = b"hello E2EE file!";
        let encrypted = encrypt_bytes(&key, plaintext).unwrap();
        assert_eq!(encrypted.len(), IV_LEN + plaintext.len() + 16); // +16 = GCM tag
        let decrypted = decrypt_bytes(&key, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_ivs() {
        // Two encryptions of same plaintext must produce different ciphertexts (random IV)
        let key = test_key();
        let ct1 = encrypt_bytes(&key, b"same data").unwrap();
        let ct2 = encrypt_bytes(&key, b"same data").unwrap();
        assert_ne!(ct1, ct2);
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let key = test_key();
        let mut encrypted = encrypt_bytes(&key, b"test").unwrap();
        // Flip a byte in the ciphertext (after IV)
        encrypted[IV_LEN] ^= 0xFF;
        assert!(decrypt_bytes(&key, &encrypted).is_err());
    }

    #[test]
    fn test_too_short_fails() {
        let key = test_key();
        assert!(decrypt_bytes(&key, &[0u8; 10]).is_err());
    }
}
