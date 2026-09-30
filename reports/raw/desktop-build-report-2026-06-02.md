# OpenCorde Desktop Build Report
**Date:** 2026-06-02  
**Task:** t_eaea8b8a — Desktop builds: Windows, macOS, Linux packaging  
**Builder:** Hermes (kanban-worker)

---

## Build Results Summary

| Platform | Status | Artifact | Size |
|----------|--------|----------|------|
| Linux (amd64) | ✅ Built | `opencorde-app` (binary) | 26 MB |
| Linux (amd64) | ✅ Packaged | `OpenCorde_0.1.0_amd64.deb` | 8.9 MB |
| Linux (amd64) | ⚠️ Partial | AppImage failed — `linuxdeploy` not installed | — |
| Windows (x86_64) | ✅ Cross-compiled | `opencorde-app.exe` (PE32+ GUI) | 32 MB |
| macOS | ❌ Blocked | No Darwin cross-compile target | — |

---

## Linux (x86_64-unknown-linux-gnu)

**Binary path:** `/home/mb/opencorde/client/src-tauri/target/release/opencorde-app`  
**Deb path:** `/home/mb/opencorde/client/src-tauri/target/release/bundle/deb/OpenCorde_0.1.0_amd64.deb`

- Built with: `pnpm tauri build --bundles deb,appimage`
- Binary compiled in release profile with optimizations
- Deb bundle includes: binary + desktop file + icons + MIME associations
- AppImage failed: `linuxdeploy` binary not found on this host. Install via:
  ```
  wget https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage
  chmod +x linuxdeploy-x86_64.AppImage
  ```
  Or add a GitHub Actions CI runner with `AppImage` support.

---

## Windows (x86_64-pc-windows-gnu)

**Binary path:** `/home/mb/opencorde/client/src-tauri/target/x86_64-pc-windows-gnu/release/opencorde-app.exe`

- Cross-compiled from Linux using `x86_64-w64-mingw32-gcc`
- PE32+ executable (GUI), stripped
- **Caveats:**
  - No MSI/NSIS installer produced (Tauri CLI on Linux only supports deb/rpm/appimage bundles)
  - Installer packaging requires a Windows runner (GitHub Actions `windows-latest` recommended)
  - `tauri-plugin-updater` permission issue on Windows target required temporary capability workaround. The updater plugin compiles but its `updater:default` permission is not recognized by the build script in cross-compile mode. Recommend testing on a native Windows CI runner.

---

## macOS

**Blocker:** No macOS cross-compilation targets installed.

- Required targets: `aarch64-apple-darwin` (Apple Silicon) and `x86_64-apple-darwin` (Intel)
- macOS cross-compilation from Linux requires:
  1. Apple SDKs (can be obtained via `osxcross`)
  2. Appropriate Rust targets (`rustup target add aarch64-apple-darwin x86_64-apple-darwin`)
  3. Code signing certificates for notarization
- **Recommendation:** Use GitHub Actions `macos-latest` runner for macOS builds. Add a `.github/workflows/build.yml` matrix with `ubuntu-latest`, `windows-latest`, `macos-latest`.

---

## Fixes Applied During Build

### Rust compilation fixes (`src-tauri/src/lib.rs`)

1. **tauri-plugin-deep-link API update:** `on_open_url` changed from standalone function to method on `DeepLink<R>` struct. Fixed to use `app.state::<DeepLink<Wry>>().on_open_url(...)`.
2. **`urls()` ownership:** `OpenUrlEvent::urls(self)` takes ownership. Fixed by calling once and storing result.
3. **`update.install()` no longer async:** Removed spurious `.await`.
4. **`url::Url` → `tauri::Url`:** Removed unused implicit dependency on `url` crate; used Tauri's re-export.

### Frontend import fixes

5. **`$lib/stores/friends` → `$lib/stores/friends.svelte`:** Two Svelte files imported without `.svelte` extension; the file on disk is `friends.svelte.ts` which resolves only with explicit extension. Fixed in:
   - `src/lib/components/layout/FriendItem.svelte`
   - `src/lib/components/user/UserProfilePopover.svelte`

---

## Configuration

- **Product name:** OpenCorde
- **Version:** 0.1.0
- **Identifier:** com.opencorde.app
- **Bundle targets:** all (deb, appimage, msi, nsis, dmg configured in tauri.conf.json)
- **Icons:** All required icon sizes present (32x32, 128x128, 128x128@2x, icon.ico, icon.png)
- **Frontend dist:** `../build` (static SvelteKit output, 2.4 MB)
- **Capabilities:** core, shell, notification, autostart, deep-link, updater, os

---

## Recommendations

1. **CI/CD Setup:** Create GitHub Actions workflow matrix for all three platforms
2. **Windows installer:** Build MSI/NSIS on Windows runner
3. **macOS:** Build .dmg on macOS runner with proper signing
4. **AppImage:** Install `linuxdeploy` on CI or this dev machine
5. **Updater:** Test `tauri-plugin-updater` on Windows; the `updater:default` capability may need a platform-specific capability file
