//! # Mobile Initialisation
//! Platform-specific setup for iOS and Android targets.
//!
//! ## Responsibilities
//! - Request push notification permission on app launch (iOS + Android)
//! - Provide stubs for platform features not yet implemented
//!
//! ## Usage
//! Called from `lib.rs` inside `.setup()` on mobile targets:
//! ```rust
//! #[cfg(mobile)]
//! mobile::init(app)?;
//! ```
//!
//! ## Depends On
//! - tauri::AppHandle (runtime handle)
//! - tauri_plugin_notification (permission request)

use tauri::AppHandle;
use tracing::instrument;

/// Initialise mobile-specific features.
///
/// Call this once during `.setup()` on mobile targets.
/// Currently requests push notification permission; extend here for
/// deep-link handling, biometrics, or other mobile-only capabilities.
#[instrument(skip(app))]
pub fn init(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("initialising mobile features");
    request_push_permission(app);
    Ok(())
}

/// Request push notification permission from the OS.
///
/// On iOS this triggers the native permission dialog the first time.
/// On Android 13+ (API 33) this triggers the POST_NOTIFICATIONS dialog.
/// The call is fire-and-forget; denial is handled gracefully (no crash).
fn request_push_permission(app: &AppHandle) {
    use tauri_plugin_notification::NotificationExt;

    match app.notification().request_permission() {
        Ok(state) => {
            tracing::info!(granted = ?state, "push notification permission state");
        }
        Err(e) => {
            tracing::warn!(error = %e, "push notification permission request failed");
        }
    }
}
