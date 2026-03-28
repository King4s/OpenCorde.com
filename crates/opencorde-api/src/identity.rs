//! # Mesh Identity
//! Manages this server's Ed25519 keypair for federation authentication.
//!
//! ## Lifecycle
//! - On first start: generates a new keypair, saves private key to `IDENTITY_KEY_PATH`
//! - On subsequent starts: loads existing keypair from file
//! - Public key is served at `GET /api/v1/federation/identity`
//!
//! ## Signing
//! Every outbound federation event is signed with the private key.
//! Receiving servers verify the signature using the sender's public key
//! (fetched from their mesh_peers table or live from /federation/identity).
//!
//! ## Storage
//! Private key: 32 raw bytes written to `IDENTITY_KEY_PATH` (default: `.opencorde.key`)
//! Keep this file secret and back it up — losing it breaks federation trust.
//!
//! ## Depends On
//! - ed25519-dalek 2 (keypair, signing, verification)
//! - hex (public key encoding for DB/wire)

use std::{fs, path::Path};
use ed25519_dalek::{Signature, Signer, SigningKey};
use rand_core::OsRng;

const DEFAULT_KEY_PATH: &str = ".opencorde.key";

/// Loaded Ed25519 identity for this server instance.
#[derive(Clone)]
pub struct ServerIdentity {
    signing_key: SigningKey,
    /// Hex-encoded 32-byte public key (64 chars) — safe to share publicly.
    pub public_key_hex: String,
}

impl ServerIdentity {
    /// Load existing keypair or generate a new one.
    ///
    /// Reads path from `IDENTITY_KEY_PATH` env var, falling back to `.opencorde.key`.
    /// Creates the file on first run.
    pub fn load_or_generate() -> anyhow::Result<Self> {
        let path = std::env::var("IDENTITY_KEY_PATH")
            .unwrap_or_else(|_| DEFAULT_KEY_PATH.to_string());

        let signing_key = if Path::new(&path).exists() {
            let bytes = fs::read(&path)?;
            if bytes.len() != 32 {
                anyhow::bail!(
                    "identity key file is corrupt (expected 32 bytes, got {})",
                    bytes.len()
                );
            }
            let arr: [u8; 32] = bytes.try_into().unwrap();
            SigningKey::from_bytes(&arr)
        } else {
            tracing::info!(path = %path, "no identity key found — generating new keypair");
            let key = SigningKey::generate(&mut OsRng);
            fs::write(&path, key.to_bytes())?;
            tracing::info!(path = %path, "identity key saved to disk");
            key
        };

        let public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());
        tracing::info!(public_key = %public_key_hex, "server identity loaded");

        Ok(Self { signing_key, public_key_hex })
    }

    /// Sign arbitrary bytes. Returns hex-encoded 64-byte signature.
    pub fn sign(&self, message: &[u8]) -> String {
        let sig: Signature = self.signing_key.sign(message);
        hex::encode(sig.to_bytes())
    }

    /// Verify a signature from a remote server.
    ///
    /// Returns true if the signature is valid for the given public key and message.
    pub fn verify(public_key_hex: &str, message: &[u8], signature_hex: &str) -> bool {
        use ed25519_dalek::{Verifier, VerifyingKey};
        let Ok(pk_bytes) = hex::decode(public_key_hex) else {
            return false;
        };
        let Ok(pk_arr) = pk_bytes.try_into() as Result<[u8; 32], _> else {
            return false;
        };
        let Ok(vk) = VerifyingKey::from_bytes(&pk_arr) else {
            return false;
        };
        let Ok(sig_bytes) = hex::decode(signature_hex) else {
            return false;
        };
        let Ok(sig_arr) = sig_bytes.try_into() as Result<[u8; 64], _> else {
            return false;
        };
        let sig = Signature::from_bytes(&sig_arr);
        vk.verify(message, &sig).is_ok()
    }
}
