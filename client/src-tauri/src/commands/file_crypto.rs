//! # E2EE File Encryption Tauri Commands
//!
//! Encrypt/decrypt file bytes for E2EE attachment uploads using AES-256-GCM.
//! The key is derived from the channel's MLS group epoch via `export_file_key`.
//!
//! ## Commands
//! - `crypto_encrypt_file` — Encrypt file bytes; returns encrypted blob as base64
//! - `crypto_decrypt_file` — Decrypt blob back to raw file bytes as base64
//!
//! ## Wire Protocol
//! Files are passed as base64-encoded strings (more compact than hex for binary data).
//! Encrypted format: `base64(IV[12] || AES-GCM ciphertext)`.
//!
//! ## Key Derivation
//! Key comes from MLS group exporter with label "opencorde-file".
//! Rotates automatically on every MLS epoch change (member add/remove).
//!
//! ## Depends On
//! - opencorde-crypto (file_crypto, group modules)
//! - crate::commands::crypto::CryptoState (shared provider + signer)

use base64::{Engine, engine::general_purpose::STANDARD};
use opencorde_crypto::{file_crypto, group};
use tauri::State;

use crate::commands::crypto::CryptoState;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn b64_decode(s: &str) -> Result<Vec<u8>, String> {
    STANDARD.decode(s).map_err(|e| format!("base64 decode failed: {e}"))
}

fn b64_encode(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err("hex string has odd length".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

/// Extract the 32-byte file key from a group state hex string.
fn derive_file_key(group_state_hex: &str, state: &CryptoState) -> Result<[u8; 32], String> {
    let provider = state.provider.lock().map_err(|e| e.to_string())?;
    let mls_group = group::deserialize_group(&hex_decode(group_state_hex)?)
        .map_err(|e| format!("group deserialization failed: {e:?}"))?;

    let key_vec = group::export_file_key(&mls_group, &provider)
        .map_err(|e| format!("file key export failed: {e:?}"))?;

    key_vec
        .try_into()
        .map_err(|_| "file key must be exactly 32 bytes".into())
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/// Encrypt file bytes with the channel's E2EE file key.
///
/// `group_state_hex` — hex-encoded MLS group state for the channel.
/// `file_data_b64`   — base64-encoded raw file bytes to encrypt.
///
/// Returns base64-encoded encrypted blob (`IV[12] || AES-GCM ciphertext`).
#[tauri::command]
pub fn crypto_encrypt_file(
    group_state_hex: String,
    file_data_b64: String,
    state: State<'_, CryptoState>,
) -> Result<String, String> {
    tracing::debug!("crypto_encrypt_file");

    let key = derive_file_key(&group_state_hex, &state)?;
    let plaintext = b64_decode(&file_data_b64)?;

    let encrypted = file_crypto::encrypt_bytes(&key, &plaintext)
        .map_err(|e| format!("file encryption failed: {e:?}"))?;

    tracing::info!(
        plaintext_len = plaintext.len(),
        encrypted_len = encrypted.len(),
        "file encrypted"
    );
    Ok(b64_encode(&encrypted))
}

/// Decrypt an encrypted file blob with the channel's E2EE file key.
///
/// `group_state_hex` — hex-encoded MLS group state for the channel.
/// `encrypted_b64`   — base64-encoded encrypted blob (`IV[12] || ciphertext`).
///
/// Returns base64-encoded decrypted file bytes.
#[tauri::command]
pub fn crypto_decrypt_file(
    group_state_hex: String,
    encrypted_b64: String,
    state: State<'_, CryptoState>,
) -> Result<String, String> {
    tracing::debug!("crypto_decrypt_file");

    let key = derive_file_key(&group_state_hex, &state)?;
    let encrypted = b64_decode(&encrypted_b64)?;

    let plaintext = file_crypto::decrypt_bytes(&key, &encrypted)
        .map_err(|e| format!("file decryption failed: {e:?}"))?;

    tracing::info!(
        decrypted_len = plaintext.len(),
        "file decrypted"
    );
    Ok(b64_encode(&plaintext))
}
