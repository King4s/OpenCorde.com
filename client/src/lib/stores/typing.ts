/**
 * @file Typing indicator store
 * @purpose Track who is currently typing in the current channel
 * @depends api/websocket, api/client
 */
import { writable, derived } from "svelte/store";
import { gateway } from "$lib/api/websocket";
import api from "$lib/api/client";

// Map of channel_id -> Map of user_id -> username
const typingUsers = writable<Map<string, Map<string, string>>>(new Map());

// Timers to clear typing status after 6 seconds (auto-expiry)
const typingTimers = new Map<string, ReturnType<typeof setTimeout>>();

/**
 * Initialize WebSocket listener for TypingStart events.
 * Should be called once per page load to set up the handler.
 */
export function initTypingListener(): void {
  gateway.on("TypingStart", (data: unknown) => {
    const evt = data as {
      channel_id: string;
      user_id: string;
      username: string;
    };
    typingUsers.update((map) => {
      const newMap = new Map(map);
      if (!newMap.has(evt.channel_id)) {
        newMap.set(evt.channel_id, new Map());
      }
      newMap.get(evt.channel_id)!.set(evt.user_id, evt.username || evt.user_id);
      return newMap;
    });

    // Auto-clear after 6 seconds of inactivity
    const key = `${evt.channel_id}:${evt.user_id}`;
    if (typingTimers.has(key)) clearTimeout(typingTimers.get(key));
    typingTimers.set(
      key,
      setTimeout(() => {
        typingUsers.update((map) => {
          const newMap = new Map(map);
          newMap.get(evt.channel_id)?.delete(evt.user_id);
          return newMap;
        });
        typingTimers.delete(key);
      }, 6000),
    );
  });
}

/**
 * Get a derived store of usernames currently typing in a specific channel.
 * Returns an array of usernames, empty if no one is typing.
 */
export function getTypingForChannel(channelId: string) {
  return derived(typingUsers, ($map) => {
    const users = $map.get(channelId);
    return users ? Array.from(users.values()) : [];
  });
}

let lastTypingSent = 0;

/**
 * Send a typing notification to the server.
 * Throttled to max once per 3 seconds to avoid spam.
 * Non-fatal if it fails (doesn't throw).
 */
export async function sendTyping(channelId: string): Promise<void> {
  const now = Date.now();
  if (now - lastTypingSent < 3000) return; // Throttle: max once per 3 seconds
  lastTypingSent = now;
  try {
    await api.post(`/channels/${channelId}/typing`, {});
  } catch {
    // Non-fatal: typing indicator is informational only
  }
}
