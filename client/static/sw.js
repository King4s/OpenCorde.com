/**
 * OpenCorde Service Worker
 *
 * Features:
 * - Build-time cache versioning (__BUILD_TIMESTAMP__ injected by Vite)
 * - Offline shell: serves cached shell for navigation, network-first with cache fallback
 * - Stale-bundle protection: new SW notifies clients to reload
 * - Connectivity monitoring: offline/online events broadcast to all clients
 * - Push notifications with notification click navigation
 *
 * Cache strategy:
 *   Navigations: network-first, fallback to cache → 200.html shell
 *   Same-origin assets: stale-while-revalidate (cache first, network update in background)
 *   Cross-origin: pass through (no caching)
 */

// ---- Cache identity --------------------------------------------------------
// Replaced at build time by vite-plugin-pwa-version
const BUILD = "__BUILD_TIMESTAMP__";
const CACHE_NAME = `opencorde-v${BUILD}`;

// ---- Precache core shell assets --------------------------------------------
const SHELL_ASSETS = [
  "/",
  "/200.html",
  "/index.html",
  "/manifest.webmanifest",
  "/app-icon-192.png",
  "/app-icon-512.png",
  "/apple-touch-icon.png",
];

// ---- Install ---------------------------------------------------------------
self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then(async (cache) => {
      try {
        await cache.addAll(SHELL_ASSETS);
      } catch (err) {
        console.warn("[sw] precache error", err);
      }
    }),
  );
  // Don't skip waiting yet — let the client decide when to activate the new SW.
  // The client will post SKIP_WAITING when the user clicks "Update now".
});

// ---- Activate --------------------------------------------------------------
self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      // Purge all old caches (any cache not matching current CACHE_NAME)
      const keys = await caches.keys();
      await Promise.all(
        keys.filter((key) => key !== CACHE_NAME).map((key) => caches.delete(key)),
      );
      // Take control of all clients immediately
      await self.clients.claim();
      console.log("[sw] activated", CACHE_NAME);
    })(),
  );
});

// ---- Fetch -----------------------------------------------------------------
self.addEventListener("fetch", (event) => {
  const { request } = event;
  if (request.method !== "GET") return;

  const url = new URL(request.url);

  // --- Navigation: network-first, offline shell fallback
  if (request.mode === "navigate") {
    event.respondWith(
      (async () => {
        try {
          const response = await fetch(request);
          // Cache successful navigation responses
          if (response.ok) {
            const cache = await caches.open(CACHE_NAME);
            cache.put(request, response.clone()).catch(() => {});
          }
          return response;
        } catch {
          // Offline: serve from cache or fallback to shell
          const cached = await caches.match(request);
          if (cached) return cached;
          // Try the SvelteKit SPA fallback
          const fallback = await caches.match("/200.html");
          if (fallback) return fallback;
          return caches.match("/") || new Response("Offline", { status: 503 });
        }
      })(),
    );
    return;
  }

  // --- Same-origin: stale-while-revalidate
  if (url.origin === self.location.origin) {
    event.respondWith(
      (async () => {
        const cached = await caches.match(request);
        const fetchPromise = fetch(request).then(async (response) => {
          if (response.ok) {
            const cache = await caches.open(CACHE_NAME);
            cache.put(request, response.clone()).catch(() => {});
          }
          return response;
        });

        // Return cached immediately if available, otherwise wait for network
        return cached || fetchPromise;
      })(),
    );
    return;
  }

  // --- Cross-origin: pass through (no caching)
});

// ---- Client messages -------------------------------------------------------
self.addEventListener("message", (event) => {
  const { type } = event.data || {};

  switch (type) {
    case "SKIP_WAITING":
      // Client requested immediate activation
      self.skipWaiting();
      break;

    case "GET_STATUS":
      // Client requests current SW state
      if (event.ports && event.ports[0]) {
        event.ports[0].postMessage({
          type: "STATUS",
          cacheName: CACHE_NAME,
          state: self.registration ? "active" : "unknown",
        });
      }
      break;

    default:
      break;
  }
});

// ---- Connectivity -----------------------------------------------------------
self.addEventListener("online", () => {
  notifyClients({ type: "CONNECTIVITY", online: true });
});

self.addEventListener("offline", () => {
  notifyClients({ type: "CONNECTIVITY", online: false });
});

async function notifyClients(data) {
  const clients = await self.clients.matchAll({ type: "window" });
  for (const client of clients) {
    client.postMessage(data);
  }
}

// ---- Push notifications ----------------------------------------------------
self.addEventListener("push", (event) => {
  let payload = {};
  try {
    payload = event.data ? event.data.json() : {};
  } catch {
    payload = { body: event.data ? event.data.text() : "" };
  }

  const title = payload.title || "OpenCorde";
  const options = {
    body: payload.body || "You have a new notification.",
    icon: "/app-icon-192.png",
    badge: "/app-icon-192.png",
    data: { url: payload.url || "/servers" },
  };

  event.waitUntil(self.registration.showNotification(title, options));
});

self.addEventListener("notificationclick", (event) => {
  event.notification.close();
  const targetUrl = (event.notification && event.notification.data && event.notification.data.url) || "/servers";
  event.waitUntil(
    self.clients
      .matchAll({ type: "window", includeUncontrolled: true })
      .then((clients) => {
        for (const client of clients) {
          if ("focus" in client) {
            client.focus();
            if ("navigate" in client) {
              client.navigate(targetUrl).catch(() => {});
            }
            return;
          }
        }
        return self.clients.openWindow(targetUrl);
      }),
  );
});
