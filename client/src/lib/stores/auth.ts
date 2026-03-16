/**
 * @file Auth store — manages authentication state
 * @purpose Token storage, login/register/logout, auto-refresh
 * @depends api/client, api/types
 */
import { writable, derived } from 'svelte/store';
import api from '$lib/api/client';
import { gateway } from '$lib/api/websocket';
import type { AuthResponse, UserProfile } from '$lib/api/types';
import { browser } from '$app/environment';

function getStoredToken(): string | null {
  if (!browser) return null;
  return localStorage.getItem('opencorde_token');
}

function storeToken(token: string | null): void {
  if (!browser) return;
  if (token) {
    localStorage.setItem('opencorde_token', token);
  } else {
    localStorage.removeItem('opencorde_token');
  }
}

// Initialize from localStorage if available
const initialToken = getStoredToken();
if (initialToken) {
  api.setToken(initialToken);
}

export const accessToken = writable<string | null>(initialToken);
export const currentUser = writable<UserProfile | null>(null);
export const isAuthenticated = derived(accessToken, ($token) => $token !== null);

// Sync token changes to localStorage
accessToken.subscribe((token) => {
  storeToken(token);
  api.setToken(token);
});

/** Restore user profile from stored token on app startup. */
export async function restoreSession(): Promise<boolean> {
  const token = getStoredToken();
  if (!token) return false;
  try {
    api.setToken(token);
    const profile = await api.get<UserProfile>('/users/@me');
    accessToken.set(token);
    currentUser.set(profile);
    gateway.connect(token);
    return true;
  } catch {
    logout();
    return false;
  }
}

export async function login(email: string, password: string): Promise<void> {
  const res = await api.post<AuthResponse>('/auth/login', { email, password });
  accessToken.set(res.access_token);
  const profile = await api.get<UserProfile>('/users/@me');
  currentUser.set(profile);
  gateway.connect(res.access_token);
}

export async function register(username: string, email: string, password: string): Promise<void> {
  const res = await api.post<AuthResponse>('/auth/register', { username, email, password });
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

export function logout(): void {
  accessToken.set(null);
  currentUser.set(null);
  gateway.disconnect();
}
