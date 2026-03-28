//! OpenCorde desktop application runtime.
//!
//! Initializes Tauri 2.0 app with:
//! - System tray icon with context menu
//! - Plugin registration (shell, notification, autostart, deep-link, os)
//! - IPC command registration (auth, settings, crypto)
//! - Window management (show/hide to tray on close)
//! - Deep-link handling via tauri-plugin-deep-link

mod commands;
#[cfg(mobile)]
mod mobile;
mod notifications;
mod tray;

use tracing_subscriber::EnvFilter;

/// Main application entry point. Builds and runs the Tauri app.
pub fn run() {
    // Initialize structured logging. Level defaults to INFO; override with RUST_LOG.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let app = tauri::Builder::default()
        // Register IPC command handlers for frontend invocation
        .invoke_handler(tauri::generate_handler![
            commands::auth::store_token,
            commands::auth::get_token,
            commands::auth::delete_token,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::get_platform,
            commands::crypto::crypto_init,
            commands::crypto::crypto_create_group,
            commands::crypto::crypto_add_member,
            commands::crypto::crypto_process_welcome,
            commands::crypto::crypto_encrypt,
            commands::crypto::crypto_decrypt,
            commands::crypto::crypto_export_voice_key,
            commands::file_crypto::crypto_encrypt_file,
            commands::file_crypto::crypto_decrypt_file,
        ])
        // E2EE session state: one OpenMLS provider per app process
        .manage(commands::crypto::CryptoState::default())
        // Register Tauri plugins for system integration
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_os::init())
        // Set up main window and tray
        .setup(|app| {
            tracing::info!("setting up OpenCorde desktop app");

            // Mobile-specific initialisation (push permission, etc.)
            #[cfg(mobile)]
            mobile::init(app.handle())?;

            // Build and initialize system tray (desktop only)
            #[cfg(not(mobile))]
            {
                let _tray = tray::build_tray(app.handle())?;
                // Request notification permission
                notifications::request_permission(app.handle().clone());
            }

            Ok(())
        })
        // Window close handler: hide to tray instead of quit
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                window.hide().ok();
                api.prevent_close();
            }
        })
        .build(tauri::generate_context!())
        .expect("error building tauri application");

    app.run(|_app_handle, _event| {});
}
