/**
 * @file Message store — manages messages for current channel
 * @purpose Fetch, send, paginate, edit, delete, react to messages; WebSocket updates
 * @depends api/client, api/types, api/websocket, stores/e2ee
 */
import { writable, get } from "svelte/store";
import api from "$lib/api/client";
import { gateway } from "$lib/api/websocket";
import type { Message, ReactionCount, Attachment } from "$lib/api/types";
import {
  getGroupState,
  requireChannelGroupState,
  requirePlaintextChannel,
  sendEncryptedMessage,
  cachedMessage,
  joinE2EEGroup,
  isNativeCrypto,
  e2eeGroupStates,
} from "./e2ee";
import { toastError } from "$lib/stores/toasts.svelte";
import { browser } from "$app/environment";

// Track channels that have already shown a decryption-failure toast (one per channel per session)
const decryptionToastShown = new Set<string>();

function isTauri(): boolean {
  return (
    browser && typeof window !== "undefined" && "__TAURI_INTERNALS__" in window
  );
}

export const messages = writable<Message[]>([]);
export const loading = writable(false);
export const hasMore = writable(true);

let currentChannelId: string | null = null;

async function decryptContent(
  channelId: string,
  content: string,
  messageId: string,
): Promise<string> {
  try {
    return await cachedMessage(channelId, messageId, content);
  } catch (error) {
    if (!decryptionToastShown.has(channelId)) {
      decryptionToastShown.add(channelId);
      toastError((error as Error).message, "Encryption recovery");
    }
    return "[Encrypted message unavailable — unlock or rejoin this channel]";
  }
}

export async function fetchMessages(
  channelId: string,
  before?: string,
): Promise<void> {
  currentChannelId = channelId;
  loading.set(true);
  const params = new URLSearchParams();
  if (before) params.set("before", before);
  params.set("limit", "50");
  const path = `/channels/${channelId}/messages?${params}`;
  const list = await api.get<Message[]>(path);

  if (
    isNativeCrypto() &&
    (getGroupState(channelId) ||
      list.some((message) => message.content.startsWith("enc:")))
  ) {
    try {
      await joinE2EEGroup(channelId);
    } catch (error) {
      toastError((error as Error).message, "Encryption recovery");
    }
  }
  const finalList = await Promise.all(
    list.map(async (message) => ({
      ...message,
      content: await decryptContent(channelId, message.content, message.id),
    })),
  );

  if (before) {
    messages.update((existing) => [...finalList, ...existing]);
  } else {
    messages.set(finalList);
  }
  hasMore.set(list.length === 50);
  loading.set(false);
}

export async function sendMessage(
  channelId: string,
  content: string,
  replyToId?: string,
  attachments?: Attachment[],
): Promise<void> {
  // Synchronization and durable outbox publication happen inside native ownership.
  if (requireChannelGroupState(channelId)) {
    await sendEncryptedMessage(channelId, content, replyToId, attachments);
    await fetchMessages(channelId);
    return;
  }
  const body: Record<string, unknown> = { content };
  if (replyToId) body.reply_to_id = replyToId;
  if (attachments && attachments.length > 0) body.attachments = attachments;
  const msg = await api.post<Message>(`/channels/${channelId}/messages`, body);
  messages.update((list) => [...list, msg]);
}

export async function editMessage(
  messageId: string,
  content: string,
): Promise<void> {
  const message = get(messages).find((entry) => entry.id === messageId);
  if (!message) throw new Error("Message security information is unavailable.");
  requirePlaintextChannel(message.channel_id, "Editing messages");
  const updated = await api.patch<Message>(`/messages/${messageId}`, {
    content,
  });
  messages.update((list) =>
    list.map((m) => (m.id === messageId ? { ...m, ...updated } : m)),
  );
}

export async function deleteMessage(messageId: string): Promise<void> {
  await api.delete(`/messages/${messageId}`);
  messages.update((list) => list.filter((m) => m.id !== messageId));
}

/** Toggle emoji reaction on a message. Refetches reactions after to get accurate `reacted` flag. */
export async function toggleReaction(
  messageId: string,
  emoji: string,
  currentlyReacted: boolean,
): Promise<void> {
  const encoded = encodeURIComponent(emoji);
  if (currentlyReacted) {
    await api.delete(`/messages/${messageId}/reactions/${encoded}`);
  } else {
    await api.put(`/messages/${messageId}/reactions/${encoded}`);
  }
  // Refetch to get accurate count + reacted flag for current user
  const reactions = await api.get<ReactionCount[]>(
    `/messages/${messageId}/reactions`,
  );
  messages.update((list) =>
    list.map((m) => (m.id === messageId ? { ...m, reactions } : m)),
  );
}

let listenersInitialized = false;
export function initMessageListener(): void {
  if (listenersInitialized) return;
  listenersInitialized = true;
  gateway.on("E2eeSync", (data: unknown) => {
    const event = data as { channel_id: string };
    if (
      isNativeCrypto() &&
      (event.channel_id === currentChannelId || getGroupState(event.channel_id))
    ) {
      void joinE2EEGroup(event.channel_id)
        .then(() =>
          event.channel_id === currentChannelId
            ? fetchMessages(event.channel_id)
            : undefined,
        )
        .catch((error) =>
          toastError((error as Error).message, "Encryption recovery"),
        );
    }
  });
  gateway.on("Ready", () => {
    if (isNativeCrypto()) {
      for (const channelId of get(e2eeGroupStates).keys()) {
        if (channelId !== currentChannelId)
          void joinE2EEGroup(channelId).catch((error) =>
            toastError((error as Error).message),
          );
      }
    }
    if (currentChannelId && isNativeCrypto()) {
      void fetchMessages(currentChannelId).catch((error) =>
        toastError((error as Error).message),
      );
    }
  });
  gateway.on("MessageCreate", (data: unknown) => {
    const event = data as { message: Message };
    if (currentChannelId && event.message.channel_id === currentChannelId) {
      const cid = currentChannelId;
      // Decrypt if E2EE
      if (
        isTauri() &&
        event.message.content.startsWith("enc:") &&
        getGroupState(cid)
      ) {
        joinE2EEGroup(cid)
          .then(() =>
            decryptContent(cid, event.message.content, event.message.id),
          )
          .then((plaintext) => {
            messages.update((list) => [
              ...list.filter((message) => message.id !== event.message.id),
              { ...event.message, content: plaintext },
            ]);
          });
      } else {
        messages.update((list) => [...list, event.message]);
      }
    }
  });

  gateway.on("MessageUpdate", (data: unknown) => {
    const event = data as { message: Message };
    if (event.message.channel_id === currentChannelId) {
      messages.update((list) =>
        list.map((m) =>
          m.id === event.message.id ? { ...m, ...event.message } : m,
        ),
      );
    }
  });

  gateway.on("MessageDelete", (data: unknown) => {
    const event = data as { channel_id: string; message_id: string };
    if (event.channel_id === currentChannelId) {
      messages.update((list) => list.filter((m) => m.id !== event.message_id));
    }
  });

  gateway.on("ReactionAdd", (data: unknown) => {
    const event = data as {
      channel_id: string;
      message_id: string;
      emoji: string;
    };
    if (event.channel_id !== currentChannelId) return;
    messages.update((list) =>
      list.map((m) => {
        if (m.id !== event.message_id) return m;
        const reactions = [...(m.reactions ?? [])];
        const idx = reactions.findIndex((r) => r.emoji === event.emoji);
        if (idx >= 0) {
          reactions[idx] = {
            ...reactions[idx],
            count: reactions[idx].count + 1,
          };
        } else {
          reactions.push({ emoji: event.emoji, count: 1, reacted: false });
        }
        return { ...m, reactions };
      }),
    );
  });

  gateway.on("ReactionRemove", (data: unknown) => {
    const event = data as {
      channel_id: string;
      message_id: string;
      emoji: string;
    };
    if (event.channel_id !== currentChannelId) return;
    messages.update((list) =>
      list.map((m) => {
        if (m.id !== event.message_id) return m;
        const reactions = (m.reactions ?? [])
          .map((r) =>
            r.emoji === event.emoji ? { ...r, count: r.count - 1 } : r,
          )
          .filter((r) => r.count > 0);
        return { ...m, reactions };
      }),
    );
  });
}
