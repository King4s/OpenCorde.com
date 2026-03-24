/**
 * @file Channel store — manages channels for current server
 * @purpose Fetch, create, select channels
 * @depends api/client, api/types
 */
import { writable, derived } from 'svelte/store';
import api from '$lib/api/client';
import type { Channel } from '$lib/api/types';

export const channels = writable<Channel[]>([]);
export const currentChannelId = writable<string | null>(null);
export const currentChannel = derived(
  [channels, currentChannelId],
  ([$channels, $id]) => $channels.find(c => c.id === $id) ?? null
);

export const textChannels = derived(channels, ($ch) => $ch.filter(c => c.channel_type === 0));
export const voiceChannels = derived(channels, ($ch) => $ch.filter(c => c.channel_type === 1));
export const stageChannels = derived(channels, ($ch) => $ch.filter(c => c.channel_type === 3));

export async function fetchChannels(serverId: string): Promise<void> {
  const list = await api.get<Channel[]>(`/servers/${serverId}/channels`);
  channels.set(list);
}

export async function createChannel(serverId: string, name: string, channelType: number = 0): Promise<Channel> {
  const channel = await api.post<Channel>(`/servers/${serverId}/channels`, { name, channel_type: channelType });
  channels.update(list => [...list, channel]);
  return channel;
}

export function selectChannel(id: string): void {
  currentChannelId.set(id);
}
