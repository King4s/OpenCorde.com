/**
 * @file Server store — manages server list state
 * @purpose Fetch, create, select servers
 * @depends api/client, api/types
 */
import { writable, derived } from 'svelte/store';
import api from '$lib/api/client';
import { gateway } from '$lib/api/websocket';
import type { Server } from '$lib/api/types';

export const servers = writable<Server[]>([]);
export const currentServerId = writable<string | null>(null);
export const currentServer = derived(
  [servers, currentServerId],
  ([$servers, $id]) => $servers.find(s => s.id === $id) ?? null
);

export async function fetchServers(): Promise<void> {
  const list = await api.get<Server[]>('/servers');
  servers.set(list);
}

export async function createServer(name: string, description?: string): Promise<Server> {
  const server = await api.post<Server>('/servers', { name, description });
  servers.update(list => [...list, server]);
  return server;
}

export function selectServer(id: string): void {
  currentServerId.set(id);
}

export function initServerListeners(): void {
  gateway.on('ServerUpdate', (data: unknown) => {
    const event = data as { server: Server };
    servers.update(list => list.map(s => s.id === event.server.id ? event.server : s));
  });
}
