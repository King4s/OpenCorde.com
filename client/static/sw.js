/**
 * @file OpenCorde Push Notification Service Worker
 * @purpose Receives Web Push messages from the backend and displays
 *          browser notifications when the tab is in the background.
 *
 * Registered by /src/lib/sw.ts via navigator.serviceWorker.register('/sw.js').
 * The push payload is JSON: { title: string, body: string }.
 */

'use strict';

// ---------------------------------------------------------------------------
// Push event — show a notification
// ---------------------------------------------------------------------------

self.addEventListener('push', (event) => {
	if (!event.data) return;

	let title = 'OpenCorde';
	let body = 'You have a new notification.';
	let icon = '/favicon.png';

	try {
		const payload = event.data.json();
		if (payload.title) title = payload.title;
		if (payload.body) body = payload.body;
	} catch (_) {
		body = event.data.text();
	}

	event.waitUntil(
		self.registration.showNotification(title, {
			body,
			icon,
			badge: icon,
			tag: 'opencorde-push'
		})
	);
});

// ---------------------------------------------------------------------------
// Notification click — focus or open the app tab
// ---------------------------------------------------------------------------

self.addEventListener('notificationclick', (event) => {
	event.notification.close();

	event.waitUntil(
		clients
			.matchAll({ type: 'window', includeUncontrolled: true })
			.then((windowClients) => {
				for (const client of windowClients) {
					if ('focus' in client) return client.focus();
				}
				if (clients.openWindow) return clients.openWindow('/');
			})
	);
});
