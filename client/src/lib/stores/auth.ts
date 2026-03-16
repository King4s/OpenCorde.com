/**
 * @file Auth store — manages authentication state
 * @purpose Token storage, login/register/logout, auto-refresh
 * @depends api/client, api/types
 */
import { writable, derived } from 'svelte/store';
import api from '$lib/api/client';
import { gateway } from '$lib/api/websocket';
import type { AuthResponse, UserProfile } from '$lib/api/types';

export const accessToken = writable<string | null>(null);
export const currentUser = writable<UserProfile | null>(null);
export const isAuthenticated = derived(accessToken, ($token) => $token !== null);

export async function login(email: string, password: string): Promise<void> {
  const res = await api.post<AuthResponse>('/auth/login', { email, password });
  accessToken.set(res.access_token);
  api.setToken(res.access_token);
  const profile = await api.get<UserProfile>('/users/@me');
  currentUser.set(profile);
  gateway.connect(res.access_token);
}

export async function register(username: string, email: string, password: string): Promise<void> {
  const res = await api.post<AuthResponse>('/auth/register', { username, email, password });
  accessToken.set(res.access_token);
  api.setToken(res.access_token);
  const profile = await api.get<UserProfile>('/users/@me');
  currentUser.set(profile);
  gateway.connect(res.access_token);
}

export async function refreshToken(): Promise<void> {
  try {
    const res = await api.post<AuthResponse>('/auth/refresh');
    accessToken.set(res.access_token);
    api.setToken(res.access_token);
  } catch {
    logout();
  }
}

export function logout(): void {
  accessToken.set(null);
  currentUser.set(null);
  api.setToken(null);
  gateway.disconnect();
}
