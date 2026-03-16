/**
 * @file Message store — manages messages for current channel
 * @purpose Fetch, send, paginate messages, WebSocket updates
 * @depends api/client, api/types, api/websocket
 */
import { writable } from 'svelte/store';
import api from '$lib/api/client';
import { gateway } from '$lib/api/websocket';
import type { Message } from '$lib/api/types';

export const messages = writable<Message[]>([]);
export const loading = writable(false);
export const hasMore = writable(true);

let currentChannelId: string | null = null;

export async function fetchMessages(channelId: string, before?: string): Promise<void> {
  currentChannelId = channelId;
  loading.set(true);
  const params = new URLSearchParams();
  if (before) params.set('before', before);
  params.set('limit', '50');
  const path = `/channels/${channelId}/messages?${params}`;
  const list = await api.get<Message[]>(path);
  if (before) {
    messages.update(existing => [...list, ...existing]);
  } else {
    messages.set(list);
  }
  hasMore.set(list.length === 50);
  loading.set(false);
}

export async function sendMessage(channelId: string, content: string): Promise<void> {
  await api.post(`/channels/${channelId}/messages`, { content });
}

export function initMessageListener(): void {
  gateway.on('MessageCreate', (data: unknown) => {
    const event = data as { message: Message };
    if (event.message.channel_id === currentChannelId) {
      messages.update(list => [...list, event.message]);
    }
  });
}
