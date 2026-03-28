# OpenCorde Desktop Icons

This directory should contain platform-specific application icons for Tauri packaging.

## Required Icons

- `32x32.png` — Icon for taskbars and small contexts
- `128x128.png` — Icon for application menus and installer
- `128x128@2x.png` — Retina version for high-DPI displays
- `icon.icns` — macOS application icon (Apple Icon Image)
- `icon.ico` — Windows application icon
- `icon.png` — Generic icon for Linux and tray

## Generating Icons

To generate icon files from a source image:

```bash
pnpm tauri icon path/to/source-icon.png
```

This command converts a source image (PNG, JPEG, or SVG) to all required platform formats.

## Icon Guidelines

- Source image should be at least 512×512 pixels
- Preferably square aspect ratio
- PNG or JPEG format recommended
- Should look good at small scales (32×32) and large (512×512)

## Current Status

Icons are not yet generated. To proceed:

1. Prepare a source icon file
2. Run `pnpm tauri icon path/to/icon.png` from the client directory
3. Icons will be placed in this directory automatically
