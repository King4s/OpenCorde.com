/**
 * @file Channel store — manages channels for current server
 * @purpose Fetch, create, select channels
 * @depends api/client, api/types
 */
import { writable, derived } from 'svelte/store';
import api from '$lib/api/client';
import { gateway } from '$lib/api/websocket';
import type { Channel } from '$lib/api/types';

let activeSpaceId: string | null = null;

export const channels = writable<Channel[]>([]);
export const currentChannelId = writable<string | null>(null);
export const currentChannel = derived(
  [channels, currentChannelId],
  ([$channels, $id]) => $channels.find(c => c.id === $id) ?? null
);

export const textChannels = derived(channels, ($ch) => $ch.filter(c => c.channel_type === 0));
export const voiceChannels = derived(channels, ($ch) => $ch.filter(c => c.channel_type === 1));
export const stageChannels = derived(channels, ($ch) => $ch.filter(c => c.channel_type === 3));
export const forumChannels = derived(channels, ($ch) => $ch.filter(c => c.channel_type === 5));
export const categoryChannels = derived(channels, ($ch) =>
  $ch.filter(c => c.channel_type === 2).sort((a, b) => a.position - b.position)
);

// Maps channel_id → server_id across all servers the user has visited this session.
// Used by the server list to show unread badges.
export const channelServerIndex = writable<Map<string, string>>(new Map());

export async function fetchChannels(spaceId: string): Promise<void> {
  activeSpaceId = spaceId;
  const list = await api.get<Channel[]>(`/servers/${spaceId}/channels`);
  channels.set(list);
  // Update cross-server index so unread badges work on the server list
  channelServerIndex.update(idx => {
    const next = new Map(idx);
    for (const ch of list) next.set(ch.id, spaceId);
    return next;
  });
}

export function initChannelListeners(): void {
  gateway.on('ChannelCreate', (data: unknown) => {
    const event = data as { channel: Channel };
    if (event.channel.server_id === activeSpaceId) {
      channels.update(list => [...list, event.channel]);
    }
  });

  gateway.on('ChannelUpdate', (data: unknown) => {
    const event = data as { channel: Channel };
    channels.update(list => list.map(c => c.id === event.channel.id ? event.channel : c));
  });

  gateway.on('ChannelDelete', (data: unknown) => {
    const event = data as { channel_id: string };
    channels.update(list => list.filter(c => c.id !== event.channel_id));
  });
}

export async function createChannel(spaceId: string, name: string, channelType: number = 0): Promise<Channel> {
  const channel = await api.post<Channel>(`/servers/${spaceId}/channels`, { name, channel_type: channelType });
  channels.update(list => [...list, channel]);
  return channel;
}

export function selectChannel(id: string): void {
  currentChannelId.set(id);
}
