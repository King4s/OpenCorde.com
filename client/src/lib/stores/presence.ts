/**
 * @file Presence store — tracks online status of users
 * @purpose Subscribe to PresenceUpdate WS events, maintain online users map
 * @depends api/websocket
 */

import { writable, derived } from "svelte/store";
import { gateway } from "$lib/api/websocket";

// Map of user_id -> status ('online' | 'offline' | 'idle' | 'dnd')
export const presenceMap = writable<Map<string, string>>(new Map());

export function initPresenceListener(): void {
  gateway.on("PresenceUpdate", (data: unknown) => {
    // Backend sends { user_id: string, online: boolean }
    const evt = data as { user_id: string; online: boolean };
    presenceMap.update((map) => {
      const newMap = new Map(map);
      if (!evt.online) {
        newMap.delete(evt.user_id);
      } else {
        newMap.set(evt.user_id, "online");
      }
      return newMap;
    });
  });
}

export function isOnline(userId: string) {
  return derived(presenceMap, ($map) => $map.has(userId));
}
