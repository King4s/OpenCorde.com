/**
 * @file Space store — manages space list state
 * @purpose Fetch, create, select spaces
 * @depends api/client, api/types
 */
import { writable, derived } from "svelte/store";
import api from "$lib/api/client";
import { gateway } from "$lib/api/websocket";
import type { Space } from "$lib/api/types";

export const spaces = writable<Space[]>([]);
export const currentSpaceId = writable<string | null>(null);
export const currentSpace = derived(
  [spaces, currentSpaceId],
  ([$servers, $id]) => $servers.find((s) => s.id === $id) ?? null,
);

export { spaces as servers };
export { currentSpaceId as currentServerId };
export { currentSpace as currentServer };

export async function fetchSpaces(): Promise<void> {
  const list = await api.get<Space[]>("/servers");
  spaces.set(list);
}

export { fetchSpaces as fetchServers };

export async function createSpace(
  name: string,
  description?: string,
): Promise<Space> {
  const space = await api.post<Space>("/servers", { name, description });
  spaces.update((list) => [...list, space]);
  return space;
}

export { createSpace as createServer };

export function selectSpace(id: string): void {
  currentSpaceId.set(id);
}

export { selectSpace as selectServer };

export function initSpaceListeners(): void {
  gateway.on("ServerUpdate", (data: unknown) => {
    const event = data as { server: Space };
    spaces.update((list) =>
      list.map((s) => (s.id === event.server.id ? event.server : s)),
    );
  });
}

export { initSpaceListeners as initServerListeners };
