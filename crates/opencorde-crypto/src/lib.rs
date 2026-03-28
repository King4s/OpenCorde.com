//! # OpenCorde Crypto
//! E2EE layer using OpenMLS 0.5 (RFC 9420).
//!
//! ## Modules
//! - `error` — CryptoError type
//! - `key_package` — KeyPackage generation and TLS serialization
//! - `group` — MLS group lifecycle (create, add members, join via welcome)
//! - `encrypt` — MLS application message encryption/decryption
//! - `file_crypto` — AES-256-GCM file encryption (IV || ciphertext format)
//!
//! ## Design
//! All cryptographic operations run client-side (Tauri app or browser/WASM).
//! The server stores opaque serialized blobs only.
//!
//! ## Depends On
//! - openmls 0.5 (RFC 9420 MLS protocol)
//! - openmls_rust_crypto 0.2 (crypto backend, implements openmls_traits 0.2)

pub mod encrypt;
pub mod error;
pub mod file_crypto;
pub mod group;
pub mod key_package;
