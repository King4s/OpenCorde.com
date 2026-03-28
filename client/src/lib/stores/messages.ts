/**
 * @file Message store — manages messages for current channel
 * @purpose Fetch, send, paginate, edit, delete, react to messages; WebSocket updates
 * @depends api/client, api/types, api/websocket, stores/e2ee
 */
import { writable } from 'svelte/store';
import api from '$lib/api/client';
import { gateway } from '$lib/api/websocket';
import type { Message, ReactionCount, Attachment } from '$lib/api/types';
import { getGroupState, setGroupState } from './e2ee';
import { browser } from '$app/environment';

function isTauri(): boolean {
  return browser && typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export const messages = writable<Message[]>([]);
export const loading = writable(false);
export const hasMore = writable(true);

let currentChannelId: string | null = null;

async function decryptContent(channelId: string, content: string): Promise<string> {
  if (!content.startsWith('enc:') || !isTauri()) return content;
  const groupState = getGroupState(channelId);
  if (!groupState) return '[Encrypted message — open channel to decrypt]';
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const result = await invoke<{ plaintext: string | null; group_state_hex: string }>(
      'crypto_decrypt', { ciphertext_hex: content.slice(4), group_state_hex: groupState }
    );
    setGroupState(channelId, result.group_state_hex);
    return result.plaintext ?? '[MLS control message]';
  } catch {
    return '[Decryption failed]';
  }
}

export async function fetchMessages(channelId: string, before?: string): Promise<void> {
  currentChannelId = channelId;
  loading.set(true);
  const params = new URLSearchParams();
  if (before) params.set('before', before);
  params.set('limit', '50');
  const path = `/channels/${channelId}/messages?${params}`;
  const list = await api.get<Message[]>(path);

  // Decrypt E2EE messages
  const finalList = (isTauri() && getGroupState(channelId))
    ? await Promise.all(list.map(async (m) => ({ ...m, content: await decryptContent(channelId, m.content) })))
    : list;

  if (before) {
    messages.update(existing => [...finalList, ...existing]);
  } else {
    messages.set(finalList);
  }
  hasMore.set(list.length === 50);
  loading.set(false);
}

export async function sendMessage(channelId: string, content: string, replyToId?: string, attachments?: Attachment[]): Promise<void> {
  let finalContent = content;
  if (isTauri()) {
    const groupState = getGroupState(channelId);
    if (groupState) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const result = await invoke<{ ciphertext_hex: string; group_state_hex: string }>(
          'crypto_encrypt', { plaintext: content, group_state_hex: groupState }
        );
        setGroupState(channelId, result.group_state_hex);
        finalContent = 'enc:' + result.ciphertext_hex;
      } catch (err) {
        console.warn('[E2EE] Encryption failed, sending plaintext:', err);
      }
    }
  }
  const body: Record<string, unknown> = { content: finalContent };
  if (replyToId) body.reply_to_id = replyToId;
  if (attachments && attachments.length > 0) body.attachments = attachments;
  const msg = await api.post<Message>(`/channels/${channelId}/messages`, body);
  messages.update(list => [...list, msg]);
}

export async function editMessage(messageId: string, content: string): Promise<void> {
  const updated = await api.patch<Message>(`/messages/${messageId}`, { content });
  messages.update(list => list.map(m => m.id === messageId ? { ...m, ...updated } : m));
}

export async function deleteMessage(messageId: string): Promise<void> {
  await api.delete(`/messages/${messageId}`);
  messages.update(list => list.filter(m => m.id !== messageId));
}

/** Toggle emoji reaction on a message. Refetches reactions after to get accurate `reacted` flag. */
export async function toggleReaction(messageId: string, emoji: string, currentlyReacted: boolean): Promise<void> {
  const encoded = encodeURIComponent(emoji);
  if (currentlyReacted) {
    await api.delete(`/messages/${messageId}/reactions/${encoded}`);
  } else {
    await api.put(`/messages/${messageId}/reactions/${encoded}`);
  }
  // Refetch to get accurate count + reacted flag for current user
  const reactions = await api.get<ReactionCount[]>(`/messages/${messageId}/reactions`);
  messages.update(list =>
    list.map(m => m.id === messageId ? { ...m, reactions } : m)
  );
}

export function initMessageListener(): void {
  gateway.on('MessageCreate', (data: unknown) => {
    const event = data as { message: Message };
    if (currentChannelId && event.message.channel_id === currentChannelId) {
      const cid = currentChannelId;
      // Decrypt if E2EE
      if (isTauri() && event.message.content.startsWith('enc:') && getGroupState(cid)) {
        decryptContent(cid, event.message.content).then(plaintext => {
          messages.update(list => [...list, { ...event.message, content: plaintext }]);
        });
      } else {
        messages.update(list => [...list, event.message]);
      }
    }
  });

  gateway.on('MessageUpdate', (data: unknown) => {
    const event = data as { message: Message };
    if (event.message.channel_id === currentChannelId) {
      messages.update(list =>
        list.map(m => m.id === event.message.id ? { ...m, ...event.message } : m)
      );
    }
  });

  gateway.on('MessageDelete', (data: unknown) => {
    const event = data as { channel_id: string; message_id: string };
    if (event.channel_id === currentChannelId) {
      messages.update(list => list.filter(m => m.id !== event.message_id));
    }
  });

  gateway.on('ReactionAdd', (data: unknown) => {
    const event = data as { channel_id: string; message_id: string; emoji: string };
    if (event.channel_id !== currentChannelId) return;
    messages.update(list =>
      list.map(m => {
        if (m.id !== event.message_id) return m;
        const reactions = [...(m.reactions ?? [])];
        const idx = reactions.findIndex(r => r.emoji === event.emoji);
        if (idx >= 0) {
          reactions[idx] = { ...reactions[idx], count: reactions[idx].count + 1 };
        } else {
          reactions.push({ emoji: event.emoji, count: 1, reacted: false });
        }
        return { ...m, reactions };
      })
    );
  });

  gateway.on('ReactionRemove', (data: unknown) => {
    const event = data as { channel_id: string; message_id: string; emoji: string };
    if (event.channel_id !== currentChannelId) return;
    messages.update(list =>
      list.map(m => {
        if (m.id !== event.message_id) return m;
        const reactions = (m.reactions ?? [])
          .map(r => r.emoji === event.emoji ? { ...r, count: r.count - 1 } : r)
          .filter(r => r.count > 0);
        return { ...m, reactions };
      })
    );
  });
}
