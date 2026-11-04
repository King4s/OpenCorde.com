//! OpenCorde desktop application runtime.
//!
//! Initializes Tauri 2.0 app with:
//! - System tray icon with context menu
//! - Plugin registration (shell, notification, autostart, deep-link, os, updater)
//! - IPC command registration (auth, settings, crypto, notifications)
//! - Window management (show/hide to tray on close)
//! - Deep-link handling via tauri-plugin-deep-link
//! - Auto-updater with background check
//! - Durable crash log capture

mod commands;
#[cfg(mobile)]
mod mobile;
mod notifications;
mod tray;

use std::panic;
use tauri::Manager;
use tracing_subscriber::EnvFilter;

/// Install a panic hook that appends backtraces to a durable crash log.
///
/// Path per platform:
/// - Linux:   `~/.local/share/opencorde/crash.log`
/// - macOS:   `~/Library/Application Support/opencorde/crash.log`
/// - Windows: `%APPDATA%\opencorde\crash.log`
///
/// Entries are appended so crash history is preserved across sessions.
fn install_crash_hook(app_handle: &tauri::AppHandle) {
    let crash_path = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("crash.log");

    panic::set_hook(Box::new(move |info| {
        let timestamp = format_system_time(std::time::SystemTime::now());
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic payload".to_string()
        };
        let location = if let Some(loc) = info.location() {
            format!("{}:{}:{}", loc.file(), loc.line(), loc.column())
        } else {
            "unknown location".to_string()
        };
        let backtrace = std::backtrace::Backtrace::capture();
        let entry = format!(
            "[{timestamp}] PANIC: {payload}\nLocation: {location}\nBacktrace:\n{backtrace}\n---\n"
        );
        // Ensure parent directory exists, then append to crash.log
        if let Err(e) =
            std::fs::create_dir_all(crash_path.parent().unwrap_or(std::path::Path::new(".")))
        {
            eprintln!("[crash] failed to create crash dir: {e}");
        }
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&crash_path)
        {
            Ok(mut f) => {
                use std::io::Write;
                let _ = writeln!(f, "{entry}");
            }
            Err(e) => eprintln!("[crash] failed to open crash log: {e}"),
        }
        eprintln!("[crash] panic logged to {crash_path:?}");
    }));
}

/// Format a SystemTime as an ISO-8601-like string without external crates.
fn format_system_time(st: std::time::SystemTime) -> String {
    let duration = st.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    let days_since_epoch = secs / 86400;
    let rem_secs = secs % 86400;
    let hour = rem_secs / 3600;
    let min = (rem_secs % 3600) / 60;
    let sec = rem_secs % 60;
    let (year, month, day) = days_to_ymd(days_since_epoch);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

/// Convert days since 1970-01-01 to (year, month, day).
fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let year_days = if leap { 366 } else { 365 };
        if days < year_days {
            break;
        }
        days -= year_days;
        year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let month_lengths = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    for &ml in &month_lengths {
        if days < ml {
            break;
        }
        days -= ml;
        month += 1;
    }
    (year, month, days + 1)
}

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
            notifications::send_notification,
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        // Set up main window and tray
        .setup(|app| {
            tracing::info!("setting up OpenCorde desktop app");

            // Install durable crash logger
            install_crash_hook(app.handle());

            // Mobile-specific initialisation (push permission, etc.)
            #[cfg(mobile)]
            mobile::init(app.handle())?;

            // Desktop-specific setup
            #[cfg(not(mobile))]
            {
                // Build and initialize system tray
                let _tray = tray::build_tray(app.handle())?;

                // Request notification permission
                notifications::request_permission(app.handle().clone());

                // Trigger background update check
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = check_for_update(handle).await {
                        tracing::warn!(error = %e, "background update check failed");
                    }
                });
            }

            // Register deep-link handler for opencorde:// scheme
            #[cfg(not(mobile))]
            {
                let handle = app.handle().clone();
                let deep_link = app.state::<tauri_plugin_deep_link::DeepLink<tauri::Wry>>();
                deep_link.on_open_url(move |event| {
                    let urls = event.urls();
                    tracing::info!(urls = ?urls, "deep link received");
                    for url in &urls {
                        handle_deep_link(&handle, url);
                    }
                });
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

/// Background update check using tauri-plugin-updater.
///
/// If an update is available it is downloaded and installed silently.
#[cfg(not(mobile))]
async fn check_for_update(app: tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app.updater()?;
    if let Some(update) = updater.check().await? {
        tracing::info!(
            version = %update.version,
            "update available, downloading"
        );

        let bytes = update.download(|_chunk, _total| {}, || {}).await?;
        update.install(bytes)?;
        tracing::info!("update installed, will apply on next launch");
    } else {
        tracing::debug!("no update available");
    }
    Ok(())
}

/// Handle an `opencorde://` deep link.
///
/// Supported paths:
/// - `opencorde://invite/{code}` — navigate to invite acceptance
/// - `opencorde://server/{id}` — navigate to a server
/// - `opencorde://settings` — open settings
#[cfg(not(mobile))]
fn handle_deep_link(app: &tauri::AppHandle, url: &tauri::Url) {
    let parsed = url.clone();

    if parsed.scheme() != "opencorde" {
        tracing::warn!(scheme = parsed.scheme(), "ignoring non-opencorde deep link");
        return;
    }

    let path = parsed.path().trim_start_matches('/');
    tracing::info!(path, "handling deep link");

    // Forward deep-link path to the frontend via the main window
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.eval(&format!(
            "window.__TAURI__.event.emit('deep-link', {{ path: '{}' }});",
            path.replace('\\', "\\\\").replace('\'', "\\'")
        ));
        let _ = window.show();
        let _ = window.set_focus();
    }
}
