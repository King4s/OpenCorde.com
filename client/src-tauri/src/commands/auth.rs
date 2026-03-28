//! Token storage and retrieval via OS keychain.
//!
//! Commands:
//! - store_token: Save JWT to OS keychain (service="opencorde", username=token_type)
//! - get_token: Retrieve token from keychain
//! - delete_token: Remove token from keychain
//!
//! The keyring crate provides cross-platform access to:
//! - Windows: Credential Manager
//! - macOS: Keychain
//! - Linux: Secret Service (requires libsecret)

use keyring::Entry;

/// Store authentication token in OS keychain.
///
/// Arguments:
/// - token: JWT or auth token string
/// - token_type: Identifier (e.g. "access_token", "refresh_token")
///
/// Returns an error string if keychain access fails.
#[tauri::command]
pub fn store_token(token: String, token_type: String) -> Result<(), String> {
    let entry = Entry::new("opencorde", &token_type)
        .map_err(|e| format!("failed to create keyring entry: {}", e))?;

    entry
        .set_password(&token)
        .map_err(|e| format!("failed to store token: {}", e))?;

    tracing::info!(token_type, "token stored in keychain");
    Ok(())
}

/// Retrieve authentication token from OS keychain.
///
/// Arguments:
/// - token_type: Identifier (e.g. "access_token", "refresh_token")
///
/// Returns Option<String>: Some(token) if found, None if not found.
/// Returns error string if keychain access fails.
#[tauri::command]
pub fn get_token(token_type: String) -> Result<Option<String>, String> {
    let entry = Entry::new("opencorde", &token_type)
        .map_err(|e| format!("failed to create keyring entry: {}", e))?;

    match entry.get_password() {
        Ok(token) => {
            tracing::debug!(token_type, "token retrieved from keychain");
            Ok(Some(token))
        }
        Err(keyring::Error::NoEntry) => {
            tracing::debug!(token_type, "token not found in keychain");
            Ok(None)
        }
        Err(e) => Err(format!("failed to retrieve token: {}", e)),
    }
}

/// Delete authentication token from OS keychain.
///
/// Arguments:
/// - token_type: Identifier (e.g. "access_token", "refresh_token")
///
/// Returns an error string if keychain access fails.
#[tauri::command]
pub fn delete_token(token_type: String) -> Result<(), String> {
    let entry = Entry::new("opencorde", &token_type)
        .map_err(|e| format!("failed to create keyring entry: {}", e))?;

    entry
        .delete_credential()
        .map_err(|e| format!("failed to delete token: {}", e))?;

    tracing::info!(token_type, "token deleted from keychain");
    Ok(())
}
