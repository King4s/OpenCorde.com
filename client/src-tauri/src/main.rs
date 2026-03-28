//! Entry point for OpenCorde desktop application.
//!
//! This binary delegates to the lib.rs entry point. Tauri requires a lightweight
//! main.rs that calls the application builder from the library.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    opencorde_app_lib::run()
}
