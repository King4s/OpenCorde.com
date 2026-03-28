//! # MLS Group Management
//! OpenMLS group (E2EE channel) lifecycle: creation, member addition, welcome handling.
//!
//! An MLS group represents an E2EE-enabled channel. Members manage group state
//! collaboratively using MLS commit/proposal mechanisms.
//!
//! ## openmls 0.5 API notes
//! - `MlsGroupConfig` is used for both create and join (not MlsGroupCreateConfig/JoinConfig)
//! - `add_members` returns `(MlsMessageOut, MlsMessageOut, Option<GroupInfo>)`;
//!   `into_welcome()` on the second is test-only — use `to_bytes()` and pass bytes around
//! - `MlsMessageIn::extract()` → `MlsMessageInBody::Welcome(w)` to get Welcome from bytes
//!
//! ## Depends On
//! - openmls 0.5 (MlsGroup, MlsGroupConfig, MlsMessageIn, MlsMessageInBody via prelude)
//! - openmls_basic_credential 0.2 (SignatureKeyPair)
//! - openmls_rust_crypto (provider)
//! - crate::error (CryptoError)

use crate::error::CryptoError;
use openmls::prelude::*;
use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto;
use tracing::instrument;

/// Create a new MLS group for a channel.
///
/// The creator becomes the first member. Returns the `MlsGroup` handle.
///
/// # Arguments
/// - `creator_credential` — Creator's credential with public signature key
/// - `signer` — Creator's signature key pair (private key for signing commits)
/// - `provider` — OpenMLS RustCrypto provider
#[instrument(skip(creator_credential, signer, provider))]
pub fn create_group(
    creator_credential: CredentialWithKey,
    signer: &SignatureKeyPair,
    provider: &OpenMlsRustCrypto,
) -> Result<MlsGroup, CryptoError> {
    tracing::debug!("creating new MLS group");

    // use_ratchet_tree_extension embeds the full tree in Welcome so joiners don't need it OOB
    let config = MlsGroupConfig::builder()
        .use_ratchet_tree_extension(true)
        .build();

    // arg order: (provider, signer, config, credential)
    let group = MlsGroup::new(provider, signer, &config, creator_credential)
        .map_err(|e| CryptoError::group(format!("group creation failed: {e:?}")))?;

    tracing::info!("MLS group created");
    Ok(group)
}

/// Add a member to an existing group.
///
/// Returns serialized commit bytes and welcome bytes (TLS via MlsMessageOut::to_bytes).
/// `into_welcome()` on MlsMessageOut is test-only in 0.5, so we serialize to bytes here
/// and let the recipient deserialize via `process_welcome`.
///
/// # Arguments
/// - `group` — Mutable reference to the MLS group
/// - `key_package` — New member's KeyPackage (public material)
/// - `signer` — Adder's signature key pair
/// - `provider` — OpenMLS RustCrypto provider
pub fn add_member(
    group: &mut MlsGroup,
    key_package: KeyPackage,
    signer: &SignatureKeyPair,
    provider: &OpenMlsRustCrypto,
) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    tracing::debug!("adding member to group");

    // Returns (commit: MlsMessageOut, welcome_out: MlsMessageOut, Option<GroupInfo>)
    let (commit, welcome_out, _group_info) = group
        .add_members(provider, signer, &[key_package])
        .map_err(|e| CryptoError::group(format!("add members failed: {e:?}")))?;

    // Merge the pending commit so the group state advances
    group
        .merge_pending_commit(provider)
        .map_err(|e| CryptoError::group(format!("merge commit failed: {e:?}")))?;

    let commit_bytes = commit
        .to_bytes()
        .map_err(|e| CryptoError::group(format!("commit serialization failed: {e:?}")))?;
    let welcome_bytes = welcome_out
        .to_bytes()
        .map_err(|e| CryptoError::group(format!("welcome serialization failed: {e:?}")))?;

    tracing::info!("member added to group");
    Ok((commit_bytes, welcome_bytes))
}

/// Join a group via serialized Welcome bytes received from the adder.
///
/// Deserializes the bytes to `MlsMessageIn`, extracts the `Welcome` variant via
/// `MlsMessageInBody`, then calls `MlsGroup::new_from_welcome`. The same `provider`
/// used to generate the member's key package must be passed here (key store lookup).
///
/// # Arguments
/// - `welcome_bytes` — TLS-serialized MLS Welcome message
/// - `provider` — OpenMLS RustCrypto provider (must contain member's key package private key)
pub fn process_welcome(
    welcome_bytes: &[u8],
    provider: &OpenMlsRustCrypto,
) -> Result<MlsGroup, CryptoError> {
    tracing::debug!("processing welcome message");

    // TlsDeserializeTrait is in scope via `use openmls::prelude::*`
    let mls_msg = MlsMessageIn::tls_deserialize(&mut std::io::Cursor::new(welcome_bytes))
        .map_err(|e| CryptoError::group(format!("welcome deserialization failed: {e:?}")))?;

    let welcome = match mls_msg.extract() {
        MlsMessageInBody::Welcome(w) => w,
        other => {
            return Err(CryptoError::group(format!(
                "expected Welcome message, got: {other:?}"
            )))
        }
    };

    // MlsGroupConfig is used for joining too (not MlsGroupJoinConfig)
    let config = MlsGroupConfig::default();
    let group = MlsGroup::new_from_welcome(provider, &config, welcome, None)
        .map_err(|e| CryptoError::group(format!("join from welcome failed: {e:?}")))?;

    tracing::info!("joined MLS group from welcome");
    Ok(group)
}

/// Serialize an MLS group state to JSON bytes for client-side persistence.
///
/// The group state contains private cryptographic material and MUST NOT be sent to the server.
pub fn serialize_group(group: &MlsGroup) -> Result<Vec<u8>, CryptoError> {
    serde_json::to_vec(group).map_err(CryptoError::Serialization)
}

/// Deserialize an MLS group state from JSON bytes.
pub fn deserialize_group(bytes: &[u8]) -> Result<MlsGroup, CryptoError> {
    serde_json::from_slice(bytes).map_err(CryptoError::Serialization)
}

/// Derive a 32-byte file encryption key from the current MLS epoch.
///
/// Uses MLS exporter with label "opencorde-file". The key changes on every
/// epoch transition, providing automatic forward secrecy for new file uploads.
/// Pass this key to `file_crypto::encrypt_bytes` / `decrypt_bytes`.
pub fn export_file_key(
    group: &MlsGroup,
    provider: &OpenMlsRustCrypto,
) -> Result<Vec<u8>, CryptoError> {
    group
        .export_secret(provider, "opencorde-file", b"", 32)
        .map_err(|e| CryptoError::group(format!("file key export failed: {e:?}")))
}

/// Derive a 32-byte voice encryption key from the current MLS epoch.
///
/// Uses MLS exporter with label "opencorde-voice". The key changes on every
/// epoch transition (member add/remove), providing automatic key rotation.
/// Safe to pass to LiveKit's ExternalE2EEKeyProvider as raw key material.
pub fn export_voice_key(
    group: &MlsGroup,
    provider: &OpenMlsRustCrypto,
) -> Result<Vec<u8>, CryptoError> {
    group
        .export_secret(provider, "opencorde-voice", b"", 32)
        .map_err(|e| CryptoError::group(format!("voice key export failed: {e:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_package;

    fn make_credential_and_signer(
        id: i64,
        provider: &OpenMlsRustCrypto,
    ) -> (CredentialWithKey, SignatureKeyPair) {
        let cred = Credential::new(id.to_le_bytes().to_vec(), CredentialType::Basic).unwrap();
        let signer = SignatureKeyPair::new(SignatureScheme::ED25519).unwrap();
        signer.store(provider.key_store()).unwrap();
        let cred_with_key = CredentialWithKey {
            credential: cred,
            signature_key: signer.public().into(),
        };
        (cred_with_key, signer)
    }

    #[test]
    fn test_create_group() {
        let provider = OpenMlsRustCrypto::default();
        let (cred, signer) = make_credential_and_signer(111, &provider);
        let result = create_group(cred, &signer, &provider);
        assert!(result.is_ok(), "{:?}", result.err());
    }

    #[test]
    fn test_add_member_and_welcome() {
        let provider = OpenMlsRustCrypto::default();
        let (creator_cred, creator_signer) = make_credential_and_signer(111, &provider);
        let mut group = create_group(creator_cred, &creator_signer, &provider).unwrap();

        let (member_kp, _member_signer) = key_package::generate_key_package(222, &provider).unwrap();
        let result = add_member(&mut group, member_kp, &creator_signer, &provider);
        assert!(result.is_ok(), "{:?}", result.err());
    }
}
