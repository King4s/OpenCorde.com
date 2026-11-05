//! # Key Package Generation and Management
//! OpenMLS KeyPackage creation, serialization, and deserialization.
//!
//! A KeyPackage is a bundle of cryptographic material representing a user's
//! E2EE identity for a single session. Clients upload several KeyPackages;
//! group initiators consume them to invite members.
//!
//! ## openmls 0.5 API notes
//! - `BasicCredential` has no `new()` — use `Credential::new(bytes, CredentialType::Basic)`
//! - `SignatureKeyPair::new(scheme)` takes no provider
//! - `KeyPackage::builder().build(CryptoConfig, provider, signer, credential_with_key)`
//! - `KeyPackage` has no `tls_deserialize`; use `KeyPackageIn::tls_deserialize` then `.validate()`
//!
//! ## Depends On
//! - openmls 0.5 (KeyPackage, KeyPackageIn, Credential, CryptoConfig via prelude)
//! - openmls_basic_credential 0.2 (SignatureKeyPair — implements Signer trait)
//! - openmls_rust_crypto (OpenMlsRustCrypto provider)
//! - crate::error (CryptoError)

use crate::error::CryptoError;
use openmls::prelude::*;
use openmls_basic_credential::SignatureKeyPair;
#[cfg(test)]
use openmls_rust_crypto::OpenMlsRustCrypto;
use tracing::instrument;

/// Generate a new OpenMLS KeyPackage and signature key pair for a user.
///
/// Creates a Credential (BasicCredential identity), a SignatureKeyPair,
/// stores it in the key store, and builds a KeyPackage using CryptoConfig::default().
///
/// The caller is responsible for:
/// - Uploading the serialized KeyPackage to the server
/// - Storing the SignatureKeyPair securely in local key storage
///
/// # Arguments
/// - `user_id` — User's Snowflake ID (used as identity bytes)
/// - `provider` — OpenMLS RustCrypto provider (key store used for HPKE keys)
#[instrument(skip(provider), fields(user_id = user_id))]
pub fn generate_key_package(
    user_id: i64,
    provider: &impl OpenMlsCryptoProvider,
) -> Result<(KeyPackage, SignatureKeyPair), CryptoError> {
    tracing::debug!("generating key package");

    let signer = SignatureKeyPair::new(SignatureScheme::ED25519)
        .map_err(|e| CryptoError::key_package(format!("signature key generation failed: {e:?}")))?;
    let key_package = generate_key_package_with_signer(user_id, &signer, provider)?;
    tracing::debug!("key package generated");
    Ok((key_package, signer))
}

/// Replenish a package without rotating the durable signing identity.
pub fn generate_key_package_with_signer(
    user_id: i64,
    signer: &SignatureKeyPair,
    provider: &impl OpenMlsCryptoProvider,
) -> Result<KeyPackage, CryptoError> {
    let credential = Credential::new(user_id.to_le_bytes().to_vec(), CredentialType::Basic)
        .map_err(|e| CryptoError::key_package(format!("{e:?}")))?;
    signer
        .store(provider.key_store())
        .map_err(|e| CryptoError::key_package(format!("{e:?}")))?;
    KeyPackage::builder()
        .build(
            CryptoConfig::default(),
            provider,
            signer,
            CredentialWithKey {
                credential,
                signature_key: signer.public().into(),
            },
        )
        .map_err(|e| CryptoError::key_package(format!("{e:?}")))
}

/// Serialize a KeyPackage to TLS bytes for transmission/storage.
///
/// Uses TlsSerializeTrait from openmls::prelude.
pub fn serialize_key_package(kp: &KeyPackage) -> Result<Vec<u8>, CryptoError> {
    kp.tls_serialize_detached()
        .map_err(|e| CryptoError::key_package(format!("serialization failed: {e:?}")))
}

/// Deserialize and validate a KeyPackage from TLS bytes.
///
/// Uses `KeyPackageIn::tls_deserialize` + `.validate()` — the two-step process
/// required by openmls 0.5 (KeyPackage itself has no tls_deserialize).
pub fn deserialize_key_package(
    bytes: &[u8],
    provider: &impl OpenMlsCryptoProvider,
) -> Result<KeyPackage, CryptoError> {
    let kp_in = KeyPackageIn::tls_deserialize(&mut std::io::Cursor::new(bytes))
        .map_err(|e| CryptoError::key_package(format!("deserialization failed: {e:?}")))?;
    kp_in
        .validate(provider.crypto(), ProtocolVersion::default())
        .map_err(|e| CryptoError::key_package(format!("key package validation failed: {e:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key_package() {
        let provider = OpenMlsRustCrypto::default();
        let result = generate_key_package(12345, &provider);
        assert!(result.is_ok(), "{:?}", result.err());
        let (kp, signer) = result.unwrap();
        assert_eq!(
            kp.leaf_node().credential().credential_type(),
            CredentialType::Basic
        );
        let _ = signer;
    }

    #[test]
    fn test_serialize_deserialize_key_package() {
        let provider = OpenMlsRustCrypto::default();
        let (kp, _) = generate_key_package(54321, &provider).unwrap();
        let serialized = serialize_key_package(&kp).unwrap();
        assert!(!serialized.is_empty());
        let deserialized = deserialize_key_package(&serialized, &provider).unwrap();
        let re_serialized = serialize_key_package(&deserialized).unwrap();
        assert_eq!(serialized, re_serialized);
    }
}
