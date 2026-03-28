//! Tauri IPC command handlers.
//!
//! Submodules:
//! - auth.rs: Token storage in OS keychain
//! - settings.rs: App preferences in app data directory
//! - crypto.rs: Client-side MLS E2EE (key packages, groups, encrypt/decrypt)
//! - file_crypto.rs: AES-256-GCM file encryption/decryption
//!
//! Each module exports #[tauri::command] functions invoked from the frontend via tauri.invoke().

pub mod auth;
pub mod crypto;
pub mod file_crypto;
pub mod settings;
