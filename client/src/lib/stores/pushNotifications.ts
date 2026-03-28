/**
 * @file Push notification store
 * @purpose Manages Web Push subscription lifecycle for browser clients.
 *
 * Requests Notification permission, subscribes to the browser's Push API
 * via a service worker, then POSTs the resulting PushSubscription JSON
 * to /api/v1/push/register so the backend can send Web Push messages.
 *
 * @depends navigator.serviceWorker, Notification API, /api/v1/push/*
 */

import { writable, get } from 'svelte/store';
import api from '$lib/api/client';

// ---------------------------------------------------------------------------
// Public store
// ---------------------------------------------------------------------------

/** True when push notifications are enabled for this browser session. */
export const notificationsEnabled = writable<boolean>(false);

// VAPID public key — must match VAPID_PRIVATE_KEY on the server.
// Set this via the VITE_VAPID_PUBLIC_KEY environment variable at build time.
const VAPID_PUBLIC_KEY = import.meta.env.VITE_VAPID_PUBLIC_KEY ?? '';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Convert a VAPID URL-safe base64 public key to Uint8Array for the Push API. */
function urlBase64ToUint8Array(base64String: string): Uint8Array {
	const padding = '='.repeat((4 - (base64String.length % 4)) % 4);
	const base64 = (base64String + padding).replace(/-/g, '+').replace(/_/g, '/');
	const rawData = atob(base64);
	return Uint8Array.from([...rawData].map((c) => c.charCodeAt(0)));
}

/** Retrieve (or create) a PushSubscription from the active service worker. */
async function getOrCreateSubscription(
	reg: ServiceWorkerRegistration
): Promise<PushSubscription | null> {
	const existing = await reg.pushManager.getSubscription();
	if (existing) return existing;

	if (!VAPID_PUBLIC_KEY) {
		console.warn('[push] VITE_VAPID_PUBLIC_KEY is not set — cannot create subscription');
		return null;
	}

	return reg.pushManager.subscribe({
		userVisibleOnly: true,
		applicationServerKey: urlBase64ToUint8Array(VAPID_PUBLIC_KEY) as BufferSource
	});
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Request browser push notification permission, subscribe via the Push API,
 * and register the resulting token with the OpenCorde backend.
 *
 * Safe to call multiple times — idempotent once subscribed.
 *
 * @throws never — all errors are caught and logged; store stays false on failure.
 */
export async function registerPushToken(): Promise<void> {
	if (!('serviceWorker' in navigator) || !('PushManager' in window)) {
		console.warn('[push] Service workers or Push API not supported in this browser');
		return;
	}

	// 1. Request permission
	const permission = await Notification.requestPermission();
	if (permission !== 'granted') {
		console.info('[push] Notification permission denied or dismissed');
		notificationsEnabled.set(false);
		return;
	}

	try {
		// 2. Register service worker (noop if already registered)
		const reg = await navigator.serviceWorker.register('/sw.js', { scope: '/' });
		await navigator.serviceWorker.ready;

		// 3. Get or create a PushSubscription
		const subscription = await getOrCreateSubscription(reg);
		if (!subscription) {
			console.warn('[push] Could not obtain PushSubscription');
			return;
		}

		// 4. POST the subscription JSON to the backend
		const token = JSON.stringify(subscription.toJSON());
		await api.post('/push/register', { token, platform: 'web' });

		notificationsEnabled.set(true);
		console.info('[push] Push token registered successfully');
	} catch (err) {
		console.error('[push] Failed to register push token', err);
		notificationsEnabled.set(false);
	}
}

/**
 * Unsubscribe from push notifications and remove the token from the backend.
 *
 * @throws never — all errors caught and logged.
 */
export async function unregisterPushToken(): Promise<void> {
	if (!('serviceWorker' in navigator)) return;

	try {
		const reg = await navigator.serviceWorker.getRegistration('/');
		if (!reg) {
			notificationsEnabled.set(false);
			return;
		}

		const subscription = await reg.pushManager.getSubscription();
		if (subscription) {
			const token = JSON.stringify(subscription.toJSON());
			await subscription.unsubscribe();
			await api.delete('/push/unregister', { token });
		}

		notificationsEnabled.set(false);
		console.info('[push] Push token unregistered');
	} catch (err) {
		console.error('[push] Failed to unregister push token', err);
	}
}
