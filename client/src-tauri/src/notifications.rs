//! Desktop notification utilities.
//!
//! Sends native OS notifications and requests permission.

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// Send a desktop notification with title and body.
///
/// If notification fails, logs a warning but does not error.
#[allow(dead_code)]
pub fn notify(app: &AppHandle, title: &str, body: &str) {
    match app.notification().builder().title(title).body(body).show() {
        Ok(_) => tracing::debug!(title, body, "notification sent"),
        Err(e) => tracing::warn!(error = ?e, "failed to send notification"),
    }
}

/// Request notification permission from the user.
///
/// Permission is requested via the frontend JS API (tauri-plugin-notification).
/// This Rust side just logs the intent; the frontend calls requestPermission() on load.
pub fn request_permission(_app: AppHandle) {
    tracing::info!("notification permission will be requested by frontend");
}
