/**
 * @file Service Worker registration helper
 * @purpose Registers the push-capable service worker (/sw.js),
 *          tracks update availability, network status, and exposes
 *          version info so the UI can show banners.
 *
 * The actual service worker source lives at client/static/sw.js.
 * This module handles registration lifecycle so other modules
 * do not import navigator.serviceWorker directly.
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface SwRegistrationResult {
  ok: boolean;
  registration?: ServiceWorkerRegistration;
  error?: string;
}

export interface UpdateCallback {
  (): void;
}

export interface ConnectivityCallback {
  (online: boolean): void;
}

// ---------------------------------------------------------------------------
// State (mutable — subscribers are notified on change)
// ---------------------------------------------------------------------------

let _updateAvailable = false;
let _online = typeof navigator !== "undefined" ? navigator.onLine : true;
let _pendingVersion: string | null = null;
let _activeVersion: string | null = null;
let _registration: ServiceWorkerRegistration | null = null;

// ---------------------------------------------------------------------------
// Subscribers (callbacks registered by layout for Svelte reactivity)
// ---------------------------------------------------------------------------

const _subscribers = new Set<() => void>();

function notify() {
  for (const fn of _subscribers) fn();
}

/** Subscribe to state changes. Returns an unsubscribe function. */
export function subscribe(fn: () => void): () => void {
  _subscribers.add(fn);
  fn(); // immediate first call
  return () => {
    _subscribers.delete(fn);
  };
}

// ---------------------------------------------------------------------------
// Getters
// ---------------------------------------------------------------------------

export function isUpdateAvailable() {
  return _updateAvailable;
}
export function isOnline() {
  return _online;
}
export function pendingVersion() {
  return _pendingVersion;
}
export function activeVersion() {
  return _activeVersion;
}

// ---------------------------------------------------------------------------
// Internal helpers — each mutates state AND notifies subscribers
// ---------------------------------------------------------------------------

function setUpdateAvailable(v: boolean) {
  if (_updateAvailable === v) return;
  _updateAvailable = v;
  notify();
}

function setOnline(v: boolean) {
  if (_online === v) return;
  _online = v;
  notify();
}

function setupUpdateDetection(reg: ServiceWorkerRegistration) {
  if (reg.waiting) {
    setUpdateAvailable(true);
  }

  reg.addEventListener("updatefound", () => {
    const installing = reg.installing;
    if (!installing) return;

    installing.addEventListener("statechange", () => {
      if (
        installing.state === "installed" &&
        navigator.serviceWorker.controller
      ) {
        setUpdateAvailable(true);
        console.info("[sw] update available — waiting to activate");
      }
    });
  });
}

function setupMessageListener() {
  navigator.serviceWorker.addEventListener("message", (event) => {
    const { type, online } = event.data || {};

    if (type === "CONNECTIVITY") {
      setOnline(!!online);
    }

    if (type === "STATUS" && event.data.cacheName) {
      _activeVersion = event.data.cacheName;
      notify();
    }
  });
}

function setupControllerChange() {
  navigator.serviceWorker.addEventListener("controllerchange", () => {
    console.info("[sw] controller changed — new SW active");
    setUpdateAvailable(false);
    _pendingVersion = null;
    notify();
  });
}

function setupConnectivityListeners() {
  window.addEventListener("online", () => setOnline(true));
  window.addEventListener("offline", () => setOnline(false));
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Register the application service worker and wire up listeners.
 * Idempotent — returns the existing registration if already registered.
 * Call once during app boot (e.g., in +layout.svelte onMount).
 */
export async function registerServiceWorker(): Promise<SwRegistrationResult> {
  if (!("serviceWorker" in navigator)) {
    return { ok: false, error: "Service workers not supported" };
  }

  setupMessageListener();
  setupControllerChange();
  setupConnectivityListeners();

  try {
    const registration = await navigator.serviceWorker.register("/sw.js", {
      scope: "/",
      updateViaCache: "none",
    });

    _registration = registration;
    console.info("[sw] registered", registration.scope);

    setupUpdateDetection(registration);
    queryActiveVersion(registration);

    return { ok: true, registration };
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    console.error("[sw] registration failed", message);
    return { ok: false, error: message };
  }
}

/** Activate waiting SW now (user clicked "Update now"). Reload follows. */
export function triggerUpdate() {
  if (!_registration || !_registration.waiting) return;
  _registration.waiting.postMessage({ type: "SKIP_WAITING" });
}

/** Check if a SW controller is active for this page. */
export function isServiceWorkerActive(): boolean {
  return (
    "serviceWorker" in navigator && navigator.serviceWorker.controller !== null
  );
}

/** Manually check for SW updates (useful on reconnect). */
export async function checkForUpdate(): Promise<boolean> {
  if (!_registration) return false;
  try {
    await _registration.update();
    return _updateAvailable;
  } catch {
    return false;
  }
}

/** Dismiss the update banner (user chose to defer). */
export function dismissUpdate() {
  setUpdateAvailable(false);
}

// ---------------------------------------------------------------------------
// Internal
// ---------------------------------------------------------------------------

async function queryActiveVersion(reg: ServiceWorkerRegistration) {
  if (!reg.active) return;
  try {
    const mc = new MessageChannel();
    const p = new Promise<string>((resolve) => {
      mc.port1.onmessage = (e) => resolve(e.data?.cacheName || "");
      setTimeout(() => resolve(""), 500);
    });
    reg.active.postMessage({ type: "GET_STATUS" }, [mc.port2]);
    const cacheName = await p;
    if (cacheName) {
      _activeVersion = cacheName;
      notify();
    }
  } catch {
    // Ignore
  }
}
