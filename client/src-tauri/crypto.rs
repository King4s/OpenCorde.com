//! # E2EE Tauri Commands
//! Client-side MLS encryption/decryption via opencorde-crypto.
//!
//! ## Session State
//! `CryptoState` lives in Tauri managed state for the app process lifetime.
//! It holds the OpenMLS crypto provider (in-memory key store) and the active
//! signing key pair. NOT persisted across restarts — call `crypto_init` on
//! every login to regenerate a key package and upload it to the server.
//!
//! ## Group State
//! MLS group state IS persisted: every mutating command returns the updated
//! group state as a hex string. The frontend stores it keyed by channel ID
//! and passes it back on the next call (stateless command pattern).
//!
//! ## Commands
//! - `crypto_init`            — Generate key package; store signer in session
//! - `crypto_create_group`    — Create MLS group; return initial group state
//! - `crypto_add_member`      — Add member; return commit + welcome + state
//! - `crypto_process_welcome` — Join via Welcome; return group state
//! - `crypto_encrypt`         — Encrypt message; return ciphertext + state
//! - `crypto_decrypt`         — Decrypt message; return plaintext + state
//!
//! ## Depends On
//! - opencorde-crypto (key_package, group, encrypt modules)
//! - openmls 0.5 (Credential, CredentialType, CredentialWithKey)
//! - openmls_basic_credential 0.2 (SignatureKeyPair)
//! - openmls_rust_crypto 0.2 (OpenMlsRustCrypto)

use opencorde_crypto::{encrypt, group, key_package};
use openmls::prelude::{Credential, CredentialType, CredentialWithKey};
use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

// ─── Session State ────────────────────────────────────────────────────────────

/// Tauri managed state for the E2EE crypto session.
/// One instance per app process; commands lock per call.
pub struct CryptoState {
    pub provider: Mutex<OpenMlsRustCrypto>,
    pub signer: Mutex<Option<SignatureKeyPair>>,
    pub user_id: Mutex<i64>,
}

impl Default for CryptoState {
    fn default() -> Self {
        Self {
            provider: Mutex::new(OpenMlsRustCrypto::default()),
            signer: Mutex::new(None),
            user_id: Mutex::new(0),
        }
    }
}

// ─── Return Types ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct AddMemberResult {
    /// TLS commit bytes to broadcast to existing group members.
    pub commit_hex: String,
    /// TLS Welcome bytes to send to the new member.
    pub welcome_hex: String,
    /// Updated group state — replace frontend's stored blob.
    pub group_state_hex: String,
}

#[derive(Serialize, Deserialize)]
pub struct EncryptResult {
    pub ciphertext_hex: String,
    /// Updated group state — replace frontend's stored blob.
    pub group_state_hex: String,
}

#[derive(Serialize, Deserialize)]
pub struct DecryptResult {
    /// Decrypted UTF-8 text, or null for MLS control messages.
    pub plaintext: Option<String>,
    /// Updated group state — replace frontend's stored blob.
    pub group_state_hex: String,
}

// ─── Hex Helpers ──────────────────────────────────────────────────────────────

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err("hex string has odd length".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/// Generate a KeyPackage and store the signing key in session state.
///
/// Call once after login. Upload the returned hex to the server:
/// `POST /api/v1/e2ee/key-packages` with `{ key_package: "<hex>" }`.
/// On every restart a fresh key package is generated; the server replaces the old one.
#[tauri::command]
pub fn crypto_init(user_id: i64, state: State<'_, CryptoState>) -> Result<String, String> {
    tracing::info!(user_id, "crypto_init: generating key package");

    let provider = state.provider.lock().map_err(|e| e.to_string())?;

    let (kp, signer) = key_package::generate_key_package(user_id, &provider)
        .map_err(|e| format!("key package generation failed: {e:?}"))?;

    let kp_bytes = key_package::serialize_key_package(&kp)
        .map_err(|e| format!("key package serialization failed: {e:?}"))?;

    *state.signer.lock().map_err(|e| e.to_string())? = Some(signer);
    *state.user_id.lock().map_err(|e| e.to_string())? = user_id;

    tracing::info!(user_id, kp_len = kp_bytes.len(), "crypto_init: key package ready");
    Ok(hex_encode(&kp_bytes))
}

/// Create a new MLS group for an E2EE channel.
///
/// Requires `crypto_init` to have been called first.
/// Returns the initial group state hex — store it keyed by channel ID.
#[tauri::command]
pub fn crypto_create_group(state: State<'_, CryptoState>) -> Result<String, String> {
    tracing::info!("crypto_create_group");

    let provider = state.provider.lock().map_err(|e| e.to_string())?;
    let signer_guard = state.signer.lock().map_err(|e| e.to_string())?;
    let signer = signer_guard
        .as_ref()
        .ok_or("crypto_init must be called before crypto_create_group")?;
    let user_id = *state.user_id.lock().map_err(|e| e.to_string())?;

    let credential = Credential::new(user_id.to_le_bytes().to_vec(), CredentialType::Basic)
        .map_err(|e| format!("credential error: {e:?}"))?;
    let credential_with_key = CredentialWithKey {
        credential,
        signature_key: signer.public().into(),
    };

    let mls_group = group::create_group(credential_with_key, signer, &provider)
        .map_err(|e| format!("group creation failed: {e:?}"))?;

    let state_bytes = group::serialize_group(&mls_group)
        .map_err(|e| format!("group serialization failed: {e:?}"))?;

    Ok(hex_encode(&state_bytes))
}

/// Add a member to an existing MLS group.
///
/// `member_key_package_hex` — fetched from server: `GET /api/v1/e2ee/key-packages/{user_id}`.
/// Returns commit (broadcast to group), welcome (send to new member), updated state.
#[tauri::command]
pub fn crypto_add_member(
    group_state_hex: String,
    member_key_package_hex: String,
    state: State<'_, CryptoState>,
) -> Result<AddMemberResult, String> {
    tracing::info!("crypto_add_member");

    let provider = state.provider.lock().map_err(|e| e.to_string())?;
    let signer_guard = state.signer.lock().map_err(|e| e.to_string())?;
    let signer = signer_guard.as_ref().ok_or("crypto_init must be called first")?;

    let mut mls_group = group::deserialize_group(&hex_decode(&group_state_hex)?)
        .map_err(|e| format!("group deserialization failed: {e:?}"))?;

    let member_kp = key_package::deserialize_key_package(&hex_decode(&member_key_package_hex)?, &provider)
        .map_err(|e| format!("key package deserialization failed: {e:?}"))?;

    let (commit_bytes, welcome_bytes) = group::add_member(&mut mls_group, member_kp, signer, &provider)
        .map_err(|e| format!("add member failed: {e:?}"))?;

    let updated_state = group::serialize_group(&mls_group)
        .map_err(|e| format!("group serialization failed: {e:?}"))?;

    Ok(AddMemberResult {
        commit_hex: hex_encode(&commit_bytes),
        welcome_hex: hex_encode(&welcome_bytes),
        group_state_hex: hex_encode(&updated_state),
    })
}

/// Join an MLS group via a Welcome message.
///
/// `welcome_hex` — fetched from server: `GET /api/v1/e2ee/groups/{group_id}/welcome`.
/// Returns the initial group state hex — store it keyed by channel ID.
///
/// Note: `crypto_init` must have been called in this session to populate the
/// provider key store (the HPKE private key from the key package is needed here).
#[tauri::command]
pub fn crypto_process_welcome(
    welcome_hex: String,
    state: State<'_, CryptoState>,
) -> Result<String, String> {
    tracing::info!("crypto_process_welcome");

    let provider = state.provider.lock().map_err(|e| e.to_string())?;
    let welcome_bytes = hex_decode(&welcome_hex)?;

    let mls_group = group::process_welcome(&welcome_bytes, &provider)
        .map_err(|e| format!("process welcome failed: {e:?}"))?;

    let state_bytes = group::serialize_group(&mls_group)
        .map_err(|e| format!("group serialization failed: {e:?}"))?;

    Ok(hex_encode(&state_bytes))
}

/// Encrypt a plaintext message using the current MLS group epoch.
///
/// Returns ciphertext and updated group state. The frontend MUST replace
/// its stored group state — the epoch ratchets forward on every send.
#[tauri::command]
pub fn crypto_encrypt(
    plaintext: String,
    group_state_hex: String,
    state: State<'_, CryptoState>,
) -> Result<EncryptResult, String> {
    tracing::debug!("crypto_encrypt");

    let provider = state.provider.lock().map_err(|e| e.to_string())?;
    let signer_guard = state.signer.lock().map_err(|e| e.to_string())?;
    let signer = signer_guard.as_ref().ok_or("crypto_init must be called first")?;

    let mut mls_group = group::deserialize_group(&hex_decode(&group_state_hex)?)
        .map_err(|e| format!("group deserialization failed: {e:?}"))?;

    let ciphertext =
        encrypt::encrypt_message(&mut mls_group, plaintext.as_bytes(), signer, &provider)
            .map_err(|e| format!("encryption failed: {e:?}"))?;

    let updated_state = group::serialize_group(&mls_group)
        .map_err(|e| format!("group serialization failed: {e:?}"))?;

    Ok(EncryptResult {
        ciphertext_hex: hex_encode(&ciphertext),
        group_state_hex: hex_encode(&updated_state),
    })
}

/// Decrypt an incoming MLS message.
///
/// Returns plaintext (None for control messages) and updated group state.
/// The frontend MUST replace its stored group state after every receive.
#[tauri::command]
pub fn crypto_decrypt(
    ciphertext_hex: String,
    group_state_hex: String,
    state: State<'_, CryptoState>,
) -> Result<DecryptResult, String> {
    tracing::debug!("crypto_decrypt");

    let provider = state.provider.lock().map_err(|e| e.to_string())?;
    let ciphertext = hex_decode(&ciphertext_hex)?;

    let mut mls_group = group::deserialize_group(&hex_decode(&group_state_hex)?)
        .map_err(|e| format!("group deserialization failed: {e:?}"))?;

    let plaintext_bytes = encrypt::decrypt_message(&mut mls_group, &ciphertext, &provider)
        .map_err(|e| format!("decryption failed: {e:?}"))?;

    let updated_state = group::serialize_group(&mls_group)
        .map_err(|e| format!("group serialization failed: {e:?}"))?;

    let plaintext = plaintext_bytes
        .map(|b| String::from_utf8(b).unwrap_or_else(|e| format!("<non-utf8: {e}>>")));

    Ok(DecryptResult {
        plaintext,
        group_state_hex: hex_encode(&updated_state),
    })
}
