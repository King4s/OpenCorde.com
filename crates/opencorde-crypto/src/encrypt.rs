//! # Message Encryption and Decryption
//! E2EE message encryption/decryption using MLS group state.
//!
//! All operations are in-memory. The caller is responsible for persisting
//! group state (via group::serialize_group) to disk/database between sessions.
//!
//! ## Depends On
//! - openmls 0.5 (MlsGroup, MlsMessageIn, MlsMessageInBody, ProcessedMessageContent)
//! - openmls_basic_credential 0.2 (SignatureKeyPair)
//! - openmls_rust_crypto (provider)
//! - crate::error (CryptoError)

use crate::error::CryptoError;
use openmls::prelude::*;
use openmls_basic_credential::SignatureKeyPair;
#[cfg(test)]
use openmls_rust_crypto::OpenMlsRustCrypto;
use tracing::instrument;

/// Encrypt a plaintext message using the group's current epoch key.
///
/// The group's internal state advances after every message. Both sender
/// and recipient must update their stored group state after each operation.
///
/// # Arguments
/// - `group` — Mutable MLS group (state advances on send)
/// - `plaintext` — Message bytes to encrypt
/// - `signer` — Sender's signature key pair
/// - `provider` — OpenMLS RustCrypto provider
pub fn encrypt_message(
    group: &mut MlsGroup,
    plaintext: &[u8],
    signer: &SignatureKeyPair,
    provider: &impl OpenMlsCryptoProvider,
) -> Result<Vec<u8>, CryptoError> {
    tracing::debug!(plaintext_len = plaintext.len(), "encrypting message");

    let mls_message = group
        .create_message(provider, signer, plaintext)
        .map_err(|e| CryptoError::encryption(format!("create_message failed: {e:?}")))?;

    // TlsSerializeTrait is in scope via `use openmls::prelude::*`
    let serialized = mls_message
        .tls_serialize_detached()
        .map_err(|e| CryptoError::encryption(format!("message serialization failed: {e:?}")))?;

    tracing::debug!(encrypted_len = serialized.len(), "message encrypted");
    Ok(serialized)
}

/// Decrypt an incoming MLS message.
///
/// Processes the message against the group state and returns plaintext if it is
/// an application message. Returns `None` for control messages (proposals, commits).
///
/// In openmls 0.5, `From<MlsMessageIn> for ProtocolMessage` is test-only, so we
/// extract via `MlsMessageIn::extract()` → `MlsMessageInBody` → match on variant.
///
/// # Arguments
/// - `group` — Mutable MLS group (state advances on receive)
/// - `message_bytes` — Serialized MLS message (TLS-encoded)
/// - `provider` — OpenMLS RustCrypto provider
///
/// # Returns
/// - `Ok(Some(plaintext))` — application message decrypted
/// - `Ok(None)` — control message processed (group state updated)
/// - `Err(e)` — decryption failed
#[instrument(skip(group, message_bytes, provider), fields(msg_len = message_bytes.len()))]
pub fn decrypt_message(
    group: &mut MlsGroup,
    message_bytes: &[u8],
    provider: &impl OpenMlsCryptoProvider,
) -> Result<Option<Vec<u8>>, CryptoError> {
    decrypt_inner(group, message_bytes, provider, false).map(|(_, plaintext)| plaintext)
}

/// Process a message and return its cryptographically authenticated sender identity.
pub fn decrypt_authenticated(
    group: &mut MlsGroup,
    message_bytes: &[u8],
    provider: &impl OpenMlsCryptoProvider,
) -> Result<(i64, Option<Vec<u8>>), CryptoError> {
    decrypt_inner(group, message_bytes, provider, true)
}

fn decrypt_inner(
    group: &mut MlsGroup,
    message_bytes: &[u8],
    provider: &impl OpenMlsCryptoProvider,
    require_user_identity: bool,
) -> Result<(i64, Option<Vec<u8>>), CryptoError> {
    tracing::debug!("decrypting message");

    // TlsDeserializeTrait is in scope via `use openmls::prelude::*`
    let mls_message = MlsMessageIn::tls_deserialize(&mut std::io::Cursor::new(message_bytes))
        .map_err(|e| CryptoError::decryption(format!("deserialization failed: {e:?}")))?;

    // extract() returns MlsMessageInBody; PrivateMessageIn/PublicMessageIn implement Into<ProtocolMessage>
    let protocol_message: ProtocolMessage = match mls_message.extract() {
        MlsMessageInBody::PrivateMessage(pm) => pm.into(),
        MlsMessageInBody::PublicMessage(pm) => pm.into(),
        other => {
            return Err(CryptoError::decryption(format!(
                "expected PrivateMessage or PublicMessage, got: {other:?}"
            )));
        }
    };

    let processed = group
        .process_message(provider, protocol_message)
        .map_err(|e| CryptoError::decryption(format!("process_message failed: {e:?}")))?;

    // The legacy library API supports arbitrary MLS Basic credentials. V2
    // application routing additionally requires an authenticated Snowflake.
    let sender = if require_user_identity {
        let identity: [u8; 8] = processed.credential().identity().try_into()
            .map_err(|_| CryptoError::decryption("invalid sender credential"))?;
        let sender = i64::from_le_bytes(identity);
        if sender <= 0 { return Err(CryptoError::decryption("invalid sender identity")); }
        sender
    } else { 0 };
    match processed.into_content() {
        ProcessedMessageContent::ApplicationMessage(app_msg) => {
            tracing::debug!("application message decrypted");
            Ok((sender, Some(app_msg.into_bytes())))
        }
        ProcessedMessageContent::StagedCommitMessage(commit) => {
            group
                .merge_staged_commit(provider, *commit)
                .map_err(|e| CryptoError::decryption(format!("merge commit failed: {e:?}")))?;
            tracing::debug!("staged commit merged");
            Ok((sender, None))
        }
        ProcessedMessageContent::ProposalMessage(_)
        | ProcessedMessageContent::ExternalJoinProposalMessage(_) => {
            tracing::debug!("proposal message processed");
            Ok((sender, None))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{group, key_package};

    fn make_cred_and_signer(
        id: i64,
        provider: &impl OpenMlsCryptoProvider,
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
    fn test_encrypt_decrypt_message() {
        let provider = OpenMlsRustCrypto::default();

        // Setup creator
        let (creator_cred, creator_signer) = make_cred_and_signer(222, &provider);
        let mut creator_group =
            group::create_group(creator_cred, &creator_signer, &provider).unwrap();

        // Add a second member; add_member returns (commit_bytes, welcome_bytes)
        let (member_kp, _member_signer) =
            key_package::generate_key_package(333, &provider).unwrap();
        let (_commit_bytes, welcome_bytes) =
            group::add_member(&mut creator_group, member_kp, &creator_signer, &provider).unwrap();

        // Encrypt by creator
        let plaintext = b"Hello, E2EE!";
        let ciphertext =
            encrypt_message(&mut creator_group, plaintext, &creator_signer, &provider).unwrap();
        assert!(!ciphertext.is_empty());

        // Member joins via welcome bytes (same provider — key store has member's key package)
        let mut member_group = group::process_welcome(&welcome_bytes, &provider).unwrap();

        // Member decrypts
        let decrypted = decrypt_message(&mut member_group, &ciphertext, &provider).unwrap();
        assert_eq!(decrypted, Some(plaintext.to_vec()));
    }
}
