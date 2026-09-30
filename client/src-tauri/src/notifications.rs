//! Desktop notification utilities.
//!
//! Sends native OS notifications and requests permission.
//! Exposes `send_notification` as a Tauri IPC command so the
//! frontend can trigger native notifications for in-app events.

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// IPC command: send a native desktop notification from the frontend.
///
/// Arguments:
/// - title: Notification title (e.g. server name)
/// - body: Notification body (e.g. author + message preview)
///
/// The frontend should call this via `tauri.invoke('send_notification', { title, body })`.
#[tauri::command]
pub fn send_notification(app: AppHandle, title: String, body: String) {
    notify(&app, &title, &body);
}

/// Send a desktop notification with title and body.
///
/// If notification fails, logs a warning but does not error.
pub fn notify(app: &AppHandle, title: &str, body: &str) {
    match app.notification().builder().title(title).body(body).show() {
        Ok(_) => tracing::debug!(title, body, "notification sent"),
        Err(e) => tracing::warn!(error = ?e, "failed to send notification"),
    }
}

/// Request notification permission from the user.
///
/// On desktop platforms the permission is requested via the notification
/// plugin's `request_permission` API. On failure we log and continue —
/// the frontend can also request through the JS API.
pub fn request_permission(app: AppHandle) {
    match app.notification().request_permission() {
        Ok(state) => tracing::info!(?state, "notification permission resolved"),
        Err(e) => tracing::warn!(error = ?e, "failed to request notification permission"),
    }
}
