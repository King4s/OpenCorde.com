/**
 * @file Auth store — manages authentication state
 * @purpose Token storage, login/register/logout, auto-refresh
 * @depends api/client, api/types
 *
 * Token storage strategy:
 * - In Tauri desktop app: OS keychain via `store_token` / `get_token` IPC commands
 * - In browser: localStorage (fallback)
 */
import { writable, derived } from 'svelte/store';
import api from '$lib/api/client';
import { gateway } from '$lib/api/websocket';
import type { AuthResponse, UserProfile } from '$lib/api/types';
import { browser } from '$app/environment';

// Detect Tauri context (window.__TAURI_INTERNALS__ is injected by Tauri runtime)
function isTauri(): boolean {
  return browser && typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

async function getStoredToken(): Promise<string | null> {
  if (!browser) return null;
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<string | null>('get_token', { tokenType: 'access_token' });
    } catch {
      // Fall through to localStorage
    }
  }
  return localStorage.getItem('opencorde_token');
}

async function persistToken(token: string | null): Promise<void> {
  if (!browser) return;
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      if (token) {
        await invoke('store_token', { token, tokenType: 'access_token' });
      } else {
        await invoke('delete_token', { tokenType: 'access_token' });
      }
      return;
    } catch {
      // Fall through to localStorage
    }
  }
  if (token) {
    localStorage.setItem('opencorde_token', token);
  } else {
    localStorage.removeItem('opencorde_token');
  }
}

// Synchronous localStorage read used only for initial store value (Tauri async path handled in restoreSession)
const initialToken = browser ? localStorage.getItem('opencorde_token') : null;
if (initialToken) {
  api.setToken(initialToken);
}

export const accessToken = writable<string | null>(initialToken);
export const currentUser = writable<UserProfile | null>(null);
export const isAuthenticated = derived(accessToken, ($token) => $token !== null);

// Sync token changes to storage + API client
accessToken.subscribe((token) => {
  persistToken(token); // async, fire-and-forget
  api.setToken(token);
});

/** Restore user profile from stored token on app startup. */
export async function restoreSession(): Promise<boolean> {
  // In Tauri, read from keychain (async); in browser, already seeded from localStorage above
  const token = isTauri() ? await getStoredToken() : localStorage.getItem('opencorde_token');
  if (!token) return false;
  try {
    api.setToken(token);
    const profile = await api.get<UserProfile>('/users/@me');
    accessToken.set(token);
    currentUser.set(profile);
    uploadKeyPackage(profile.id);
    gateway.connect(token);
    return true;
  } catch {
    logout();
    return false;
  }
}

export async function login(email: string, password: string, totp_code?: string): Promise<void> {
  const body: Record<string, string> = { email, password };
  if (totp_code) body.totp_code = totp_code;
  const res = await api.post<AuthResponse>('/auth/login', body);
  accessToken.set(res.access_token);
  const profile = await api.get<UserProfile>('/users/@me');
  currentUser.set(profile);
  uploadKeyPackage(profile.id);
  gateway.connect(res.access_token);
}

export async function register(username: string, email: string, password: string, inviteCode?: string): Promise<void> {
  const body = inviteCode ? { username, email, password, invite_code: inviteCode } : { username, email, password };
  const res = await api.post<AuthResponse>('/auth/register', body);
  accessToken.set(res.access_token);
  const profile = await api.get<UserProfile>('/users/@me');
  currentUser.set(profile);
  gateway.connect(res.access_token);
}

export async function refreshToken(): Promise<void> {
  try {
    const res = await api.post<AuthResponse>('/auth/refresh');
    accessToken.set(res.access_token);
  } catch {
    logout();
  }
}

async function uploadKeyPackage(userId: string): Promise<void> {
  if (!isTauri()) return;
  try {
    const { invoke: tauriInvoke } = await import('@tauri-apps/api/core');
    const kpHex = await tauriInvoke<string>('crypto_init', { user_id: parseInt(userId) });
    await api.post('/users/me/key-packages', { key_package: kpHex });
  } catch (err) {
    console.warn('[E2EE] Key package upload failed:', err);
  }
}

export function logout(): void {
  accessToken.set(null);
  currentUser.set(null);
  gateway.disconnect();
}
