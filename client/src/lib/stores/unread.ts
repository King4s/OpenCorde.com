/**
 * @file Unread message tracking store
 * @purpose Track unread message counts per channel, auto-ack on view
 * @depends api/client, api/websocket, stores/messages
 */
import { writable, derived } from 'svelte/store';
import api from '$lib/api/client';
import { gateway } from '$lib/api/websocket';

interface ReadState {
  channel_id: string;
  last_read_id: string;
  mention_count: number;
}

// Map of channel_id -> last_read_message_id
const readStates = writable<Map<string, ReadState>>(new Map());

// Map of channel_id -> unread count (exported for direct subscription)
export const unreadCounts = writable<Map<string, number>>(new Map());

let initialized = false;

export function initUnreadListener(): void {
  if (initialized) return;
  initialized = true;

  // When a message arrives, check if we need to increment unread
  gateway.on('MessageCreate', (data: unknown) => {
    const evt = data as { message: { channel_id: string; id: string } };
    const channelId = evt.message.channel_id;

    readStates.update(states => {
      const state = states.get(channelId);
      const lastReadId = state?.last_read_id ?? '0';

      // If message is newer than last read, increment unread count
      if (BigInt(evt.message.id) > BigInt(lastReadId)) {
        unreadCounts.update(counts => {
          const newCounts = new Map(counts);
          newCounts.set(channelId, (newCounts.get(channelId) ?? 0) + 1);
          return newCounts;
        });
      }

      return states;
    });
  });

  // Handle ack events from other sessions
  gateway.on('ChannelAck', (data: unknown) => {
    const evt = data as { channel_id: string; last_read_id: string };
    markChannelRead(evt.channel_id, evt.last_read_id);
  });
}

export async function loadReadStates(): Promise<void> {
  try {
    const states = await api.get<ReadState[]>('/users/@me/read-states');
    readStates.update(() => {
      const map = new Map<string, ReadState>();
      for (const s of states) {
        map.set(s.channel_id, s);
      }
      return map;
    });
  } catch (e) {
    // Non-fatal if not authenticated yet
    console.warn('Failed to load read states:', e);
  }
}

export async function ackChannel(
  channelId: string,
  lastMessageId: string
): Promise<void> {
  try {
    await api.post(`/channels/${channelId}/ack`, { message_id: lastMessageId });
    markChannelRead(channelId, lastMessageId);
  } catch (e) {
    // Non-fatal, log and continue
    console.warn(`Failed to ack channel ${channelId}:`, e);
  }
}

function markChannelRead(channelId: string, lastReadId: string): void {
  readStates.update(states => {
    const newStates = new Map(states);
    newStates.set(channelId, {
      channel_id: channelId,
      last_read_id: lastReadId,
      mention_count: 0
    });
    return newStates;
  });

  unreadCounts.update(counts => {
    const newCounts = new Map(counts);
    newCounts.delete(channelId);
    return newCounts;
  });
}

export const hasAnyUnread = derived(
  unreadCounts,
  $counts => Array.from($counts.values()).some(c => c > 0)
);

/** Map of channel_id → last_read_id (for NEW MESSAGES divider placement). */
export const lastReadIds = derived(
  readStates,
  $states => {
    const map = new Map<string, string>();
    for (const [id, s] of $states) {
      if (s.last_read_id) map.set(id, s.last_read_id);
    }
    return map;
  }
);
