//! OpenCorde desktop application runtime.
//!
//! Initializes Tauri 2.0 app with:
//! - System tray icon with context menu
//! - Plugin registration (shell, notification, autostart, deep-link, os)
//! - IPC command registration (auth, settings, crypto)
//! - Window management (show/hide to tray on close)
//! - Deep-link event forwarding to SvelteKit frontend

mod commands;
mod notifications;
mod tray;

use tauri::Manager;
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
        ])
        // E2EE session state: one OpenMLS provider per app process
        .manage(commands::crypto::CryptoState::default())
        // Register Tauri plugins for system integration
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_os::init())
        // Deep-link event handler: forward opencorde:// URLs to frontend
        .on_uri_scheme_request("opencorde", |request| {
            tracing::info!(uri = %request.uri, "deep-link received");
            // Forward to frontend via event; frontend listens on "deep-link" event
            if let Some(app_handle) = request.app_handle() {
                let url = request.uri.to_string();
                let _ = app_handle.emit("deep-link", url);
            }
        })
        // Set up main window and tray
        .setup(|app| {
            tracing::info!("setting up OpenCorde desktop app");

            // Build and initialize system tray
            let _tray = tray::build_tray(&app.handle())?;

            // Request notification permission
            notifications::request_permission(app.handle().clone());

            Ok(())
        })
        // Window close handler: hide to tray instead of quit
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { invoke, .. } = event {
                window.hide().ok();
                *invoke = false; // Prevent actual close; hide instead
            }
        })
        .build(tauri::generate_context!())
        .expect("error building tauri application");

    app.run(|_app_handle, _event| {});
}
