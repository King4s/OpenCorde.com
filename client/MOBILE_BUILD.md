# OpenCorde Mobile Build Guide

Tauri 2.0 supports building the OpenCorde client as a native iOS or Android app
using the same SvelteKit frontend. The mobile targets share all UI code; only
platform-specific init (`src-tauri/src/mobile.rs`) differs.

---

## iOS

### Prerequisites

- macOS with Xcode 15 or later
- Apple Developer account (free account works for device debugging; paid required for App Store)
- iOS device or Simulator (Simulator is sufficient for development)
- Rust iOS targets: `rustup target add aarch64-apple-ios x86_64-apple-ios`

### Setup

```bash
npm run tauri ios init
```

This generates the Xcode project under `src-tauri/gen/apple/`.

### Development (hot-reload)

```bash
npm run tauri ios dev
```

### Production build

```bash
npm run tauri ios build
```

Produces an `.ipa` archive. Sign via Xcode Organizer or `xcodebuild`.

### Push Notifications (APNS)

1. Enable Push Notifications capability in Xcode → Signing & Capabilities.
2. Generate an APNS key in Apple Developer Portal → Keys.
3. Implement `mobile::send_apns_push()` in `src-tauri/src/mobile.rs` (currently a stub).

---

## Android

### Prerequisites

- Android Studio (latest stable)
- JDK 17 (`JAVA_HOME` must point to it)
- Android NDK (install via Android Studio → SDK Manager → NDK)
- Set `ANDROID_HOME` and `NDK_HOME` environment variables
- Rust Android targets:
  ```bash
  rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android
  ```

### Setup

```bash
npm run tauri android init
```

Generates the Android project under `src-tauri/gen/android/`.

### Development (hot-reload)

```bash
npm run tauri android dev
```

### Production build

```bash
npm run tauri android build
```

Produces a signed `.apk` or `.aab` depending on the Gradle config.

### Push Notifications (FCM)

1. Create a Firebase project at https://console.firebase.google.com
2. Add your Android app (use `com.opencorde.app` as the package name).
3. Download `google-services.json` and place it in `src-tauri/gen/android/app/`.
4. Set `FCM_SERVER_KEY` in the API server's environment (`.env` or systemd unit).

---

## Bundle Identifier

The app is registered as `com.opencorde.app` (`tauri.conf.json → identifier`).
This must match:

- iOS: the App ID in your Apple Developer Portal provisioning profile
- Android: the `applicationId` in `build.gradle`

---

## Environment Variables for Push

| Variable                | Platform       | Purpose                              |
| ----------------------- | -------------- | ------------------------------------ |
| `VAPID_PRIVATE_KEY`     | Web            | VAPID private key (URL-safe base64)  |
| `VITE_VAPID_PUBLIC_KEY` | Web (frontend) | VAPID public key served to browsers  |
| `FCM_SERVER_KEY`        | Android        | Firebase server key for FCM HTTP API |
