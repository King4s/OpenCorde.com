//! App settings stored in `{appDataDir}/settings.json`.
//!
//! Commands:
//! - get_settings: Read settings.json or return defaults
//! - save_settings: Write settings to file; sync autostart if changed
//! - get_platform: Return OS name

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

/// Application settings structure.
///
/// Stored in {appDataDir}/settings.json.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSettings {
    /// UI theme: "dark" or "light"
    pub theme: String,
    /// Compact mode toggle
    pub compact_mode: bool,
    /// Desktop notifications enabled
    pub notifications_enabled: bool,
    /// Auto-launch on system startup
    pub autostart: bool,
    /// Backend server URL
    pub server_url: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            compact_mode: false,
            notifications_enabled: true,
            autostart: false,
            server_url: "https://opencorde.com".to_string(),
        }
    }
}

/// Get app settings from `{appDataDir}/settings.json`.
///
/// If the file does not exist, returns default settings.
#[tauri::command]
pub fn get_settings(app: tauri::AppHandle) -> Result<AppSettings, String> {
    let settings_path = get_settings_path(&app)?;

    if settings_path.exists() {
        let contents = std::fs::read_to_string(&settings_path)
            .map_err(|e| format!("failed to read settings: {}", e))?;
        let settings: AppSettings = serde_json::from_str(&contents)
            .map_err(|e| format!("failed to parse settings: {}", e))?;
        tracing::debug!("settings loaded from disk");
        Ok(settings)
    } else {
        tracing::debug!("settings file not found, returning defaults");
        Ok(AppSettings::default())
    }
}

/// Save app settings to `{appDataDir}/settings.json`.
///
/// If autostart setting changed, syncs it with the autostart plugin.
#[tauri::command]
pub fn save_settings(app: tauri::AppHandle, settings: AppSettings) -> Result<(), String> {
    let settings_path = get_settings_path(&app)?;

    // Create app data directory if it doesn't exist
    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create app data dir: {}", e))?;
    }

    // Write settings to file
    let json = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("failed to serialize settings: {}", e))?;
    std::fs::write(&settings_path, json)
        .map_err(|e| format!("failed to write settings: {}", e))?;

    tracing::info!(autostart = settings.autostart, "settings saved");

    // Sync autostart plugin
    if settings.autostart {
        app.autolaunch()
            .enable()
            .map_err(|e| format!("failed to enable autostart: {}", e))?;
    } else {
        app.autolaunch()
            .disable()
            .map_err(|e| format!("failed to disable autostart: {}", e))?;
    }

    Ok(())
}

/// Get the operating system name.
///
/// Returns: "linux", "macos", "windows", "ios", "android", or "unknown"
#[tauri::command]
pub fn get_platform() -> String {
    std::env::consts::OS.to_string()
}

/// Get the full path to settings.json in app data directory.
fn get_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|mut p| {
            p.push("settings.json");
            p
        })
        .map_err(|e| format!("failed to get app data dir: {}", e))
}
