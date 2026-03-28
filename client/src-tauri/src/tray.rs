//! System tray icon and context menu.
//!
//! Creates a tray icon with a menu allowing users to:
//! - Open the main window
//! - Check for updates (opens browser to releases page)
//! - Quit the application
//!
//! Tray left-click shows/focuses the main window.

use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::ShellExt;

/// Build and initialize system tray with menu.
///
/// Returns the tray icon handle.
pub fn build_tray(app: &AppHandle) -> tauri::Result<tauri::tray::TrayIcon> {
    let open_item = MenuItemBuilder::new("Open OpenCorde")
        .id("open")
        .build(app)?;

    let check_updates_item = MenuItemBuilder::new("Check for Updates")
        .id("check_updates")
        .build(app)?;

    let quit_item = MenuItemBuilder::new("Quit")
        .id("quit")
        .build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[&open_item, &check_updates_item, &quit_item])
        .build()?;

    let tray = tauri::tray::TrayIconBuilder::new()
        .menu(&menu)
        .on_menu_event(move |app, event| {
            match event.id.as_ref() {
                "open" => {
                    // Show and focus the main window
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "check_updates" => {
                    // Open releases page in browser
                    #[allow(deprecated)]
                    let _ = app.shell().open(
                        "https://github.com/opencorde/opencorde/releases",
                        None,
                    );
                }
                "quit" => {
                    // Exit the application
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            use tauri::tray::{MouseButton, TrayIconEvent};
            if let TrayIconEvent::Click { button: MouseButton::Left, .. } = event {
                // Left-click: show/focus main window
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    tracing::info!("system tray initialized");
    Ok(tray)
}
