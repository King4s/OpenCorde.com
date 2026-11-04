//! Compatibility fixture generated with registry OpenMLS 0.5.0 and Rust 1.97.1.
//! All identities and private key material here are disposable test data.
use opencorde_crypto::{encrypt, group, key_package};
use openmls::prelude::*;
use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MESSAGE: &[u8] = b"OpenMLS 0.5 baseline message";
#[derive(Serialize, Deserialize)]
struct Fixture {
    alice: Vec<u8>,
    bob: Vec<u8>,
    alice_signer: SignatureKeyPair,
    bob_signer: SignatureKeyPair,
    message: Vec<u8>,
    file_key: Vec<u8>,
    voice_key: Vec<u8>,
}

fn fixture_path() -> PathBuf {
    std::env::var_os("OPENMLS_COMPAT_FIXTURE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/openmls-0.5.0.json")
        })
}

fn read_fixture() -> Fixture {
    serde_json::from_slice(&std::fs::read(fixture_path()).unwrap()).unwrap()
}

/// Explicit fixture regeneration only. Keep the committed fixture from the
/// unpatched registry crate; generating with the patch would erase the baseline.
#[test]
#[ignore = "fixture generator; run only against the documented baseline"]
fn generate_compatibility_fixture() {
    let alice_provider = OpenMlsRustCrypto::default();
    let bob_provider = OpenMlsRustCrypto::default();
    let alice_signer = SignatureKeyPair::new(SignatureScheme::ED25519).unwrap();
    alice_signer.store(alice_provider.key_store()).unwrap();
    let credential = CredentialWithKey {
        credential: Credential::new(b"fixture-alice".to_vec(), CredentialType::Basic).unwrap(),
        signature_key: alice_signer.public().into(),
    };
    let mut alice = group::create_group(credential, &alice_signer, &alice_provider).unwrap();
    let (bob_package, bob_signer) = key_package::generate_key_package(222, &bob_provider).unwrap();
    // Cross the wire using serialized, validated key-package and welcome bytes.
    let bob_package = key_package::deserialize_key_package(
        &key_package::serialize_key_package(&bob_package).unwrap(),
        &alice_provider,
    )
    .unwrap();
    let (_, welcome) =
        group::add_member(&mut alice, bob_package, &alice_signer, &alice_provider).unwrap();
    let bob = group::process_welcome(&welcome, &bob_provider).unwrap();
    let message =
        encrypt::encrypt_message(&mut alice, MESSAGE, &alice_signer, &alice_provider).unwrap();
    let fixture = Fixture {
        alice: group::serialize_group(&alice).unwrap(),
        bob: group::serialize_group(&bob).unwrap(),
        alice_signer,
        bob_signer,
        message,
        file_key: group::export_file_key(&alice, &alice_provider).unwrap(),
        voice_key: group::export_voice_key(&alice, &alice_provider).unwrap(),
    };
    std::fs::write(fixture_path(), serde_json::to_vec(&fixture).unwrap()).unwrap();
}

#[test]
fn baseline_state_and_messages_survive_restart() {
    let fixture = read_fixture();
    // Fresh providers model a process restart, without sharing an in-memory store.
    let alice_provider = OpenMlsRustCrypto::default();
    let bob_provider = OpenMlsRustCrypto::default();
    let mut alice = group::deserialize_group(&fixture.alice).unwrap();
    let mut bob = group::deserialize_group(&fixture.bob).unwrap();
    for (state, provider) in [(&alice, &alice_provider), (&bob, &bob_provider)] {
        assert_eq!(
            group::export_file_key(state, provider).unwrap(),
            fixture.file_key
        );
        assert_eq!(
            group::export_voice_key(state, provider).unwrap(),
            fixture.voice_key
        );
    }
    assert_ne!(fixture.file_key, fixture.voice_key);
    assert_eq!(
        encrypt::decrypt_message(&mut bob, &fixture.message, &bob_provider).unwrap(),
        Some(MESSAGE.to_vec())
    );
    let reply = encrypt::encrypt_message(
        &mut bob,
        b"reply after upgrade",
        &fixture.bob_signer,
        &bob_provider,
    )
    .unwrap();
    assert_eq!(
        encrypt::decrypt_message(&mut alice, &reply, &alice_provider).unwrap(),
        Some(b"reply after upgrade".to_vec())
    );

    // Persist advanced sender/receiver ratchets, reload, and continue both ways.
    alice = group::deserialize_group(&group::serialize_group(&alice).unwrap()).unwrap();
    bob = group::deserialize_group(&group::serialize_group(&bob).unwrap()).unwrap();
    let followup = encrypt::encrypt_message(
        &mut alice,
        b"after second restart",
        &fixture.alice_signer,
        &alice_provider,
    )
    .unwrap();
    assert_eq!(
        encrypt::decrypt_message(&mut bob, &followup, &bob_provider).unwrap(),
        Some(b"after second restart".to_vec())
    );
    assert_eq!(
        group::export_file_key(&alice, &alice_provider).unwrap(),
        fixture.file_key
    );
}

#[test]
fn truncated_baseline_message_is_rejected() {
    let mut fixture = read_fixture();
    let mut bob = group::deserialize_group(&fixture.bob).unwrap();
    let provider = OpenMlsRustCrypto::default();
    fixture.message.truncate(1);
    assert!(encrypt::decrypt_message(&mut bob, &fixture.message, &provider).is_err());
}
