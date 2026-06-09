# Mobile Build Analysis — 2026-06-02

## Path Established
**Tauri 2.0 mobile** (not Capacitor). Confirmed by:
- Mobile stub at `client/src-tauri/src/mobile.rs`
- `lib.rs` conditionally includes `#[cfg(mobile)] mod mobile;` and calls `mobile::init()`
- Tauri CLI v2.10.1 available via `npx tauri`
- All Tauri plugins (notification, deep-link, os) are v2, which have mobile support

## Configuration Changes Made

### tauri.conf.json
1. Added `bundle.android`:
   - `minSdkVersion: 24`
   - `autoIncrementVersionCode: true`
   - `debugApplicationIdSuffix: ".debug"`
   - `versionCode: null` (auto-derived from semver)

2. Added `bundle.ios`:
   - `minimumSystemVersion: "14.0"`

3. Added `plugins.deep-link.mobile: []` (empty array = no custom schemes, app handles via standard intent)

### No changes needed for:
- **API base URL**: Uses `getApiBase()` → defaults to relative `/api/v1`; user can set `opencorde_server` in localStorage. Works identically on mobile.
- **Token storage**: Dual system — `localStorage` for webview + `keyring` crate for OS keychain (Android Keystore / iOS Keychain). The keyring crate is already in Cargo.toml.
- **Icons**: 15 PNG icons exist in `src-tauri/icons/` (Windows-style square logos). Tauri's `npx tauri android init` will copy the right ones into the Android project.

## Rust Targets Installed
- `aarch64-linux-android` (ARM64 — modern devices)
- `armv7-linux-androideabi` (ARMv7 — older 32-bit)
- `i686-linux-android` (x86 32-bit — emulator)
- `x86_64-linux-android` (x86_64 — emulator)

## Build Commands (Once SDK Is Available)
```bash
# First-time setup
cd /home/mb/opencorde/client
npx tauri android init   # generates gen/android/

# Development build (debug, unoptimized)
npx tauri android dev     # builds & launches on connected device/emulator

# Release build (optimized, signed)
npx tauri android build   # produces .apk or .aab in gen/android/app/build/outputs/
```

## iOS Status
Requires macOS + Xcode (≥15.0). Blocked on this Linux machine. Commands:
```bash
npx tauri ios init        # generates gen/ios/ (macOS only)
npx tauri ios dev          # builds & launches on simulator/device
npx tauri ios build        # produces .ipa
```

## Remaining Items
- [ ] Install Android SDK + NDK
- [ ] Set ANDROID_HOME, NDK_HOME, JAVA_HOME
- [ ] Run `npx tauri android init`
- [ ] Run `npx tauri android build` (debug)
- [ ] Verify APK output exists
- [ ] iOS: blocked — needs macOS
