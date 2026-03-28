//! Build script for OpenCorde desktop app.
//!
//! This script runs Tauri's build system which generates TypeScript bindings,
//! icons, and platform-specific assets.

fn main() {
    tauri_build::build()
}
