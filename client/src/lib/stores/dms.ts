/**
 * @file DM store — direct message channels + messages
 * @purpose Manage DM channel list and active DM messages
 * @depends api/client, api/websocket, api/types
 */
import { writable, derived } from "svelte/store";
import api from "$lib/api/client";
import { gateway } from "$lib/api/websocket";
import type { DmChannel, DmMessage } from "$lib/api/types";

export const dmChannels = writable<DmChannel[]>([]);
export const activeDmMessages = writable<DmMessage[]>([]);
export const dmLoading = writable(false);
export const dmUnreadCounts = writable<Map<string, number>>(new Map());
export const hasAnyDmUnread = derived(dmUnreadCounts, ($counts) =>
  Array.from($counts.values()).some((c) => c > 0),
);

let activeDmId: string | null = null;
let messageRequest = 0;
let sessionGeneration = 0;
let sessionKey = readSessionKey();
let listenerOwners = 0;
const seenMessageIds = new Set<string>();
const unreadMessageIds = new Map<string, string>();

function readSessionKey(): string | null {
  if (typeof localStorage === "undefined") return null;
  const token = localStorage.getItem("opencorde_token");
  return token
    ? JSON.stringify([localStorage.getItem("opencorde_server"), token])
    : null;
}

/** Leave the conversation and invalidate its pending history requests. */
export function clearActiveDm(): void {
  activeDmId = null;
  messageRequest++;
  activeDmMessages.set([]);
  dmLoading.set(false);
}

/** Drop account-scoped DM state when the session ends or changes. */
export function resetDmState(): void {
  sessionGeneration++;
  sessionKey = readSessionKey();
  clearActiveDm();
  seenMessageIds.clear();
  unreadMessageIds.clear();
  dmChannels.set([]);
  dmUnreadCounts.set(new Map());
}

function currentSession(): number {
  if (readSessionKey() !== sessionKey) resetDmState();
  return sessionGeneration;
}

function rememberMessage(message: DmMessage): boolean {
  if (seenMessageIds.has(message.id)) return false;
  seenMessageIds.add(message.id);
  // Retain a bounded replay window across conversation/layout changes.
  if (seenMessageIds.size > 2048) {
    const oldest = seenMessageIds.values().next().value!;
    seenMessageIds.delete(oldest);
    unreadMessageIds.delete(oldest);
  }
  return true;
}

function uniqueMessages(messages: DmMessage[]): DmMessage[] {
  const ids = new Set<string>();
  return messages.filter((message) => {
    if (ids.has(message.id)) return false;
    ids.add(message.id);
    return true;
  });
}

function clearDmUnread(dmId: string): void {
  for (const [id, channelId] of unreadMessageIds) {
    if (channelId === dmId) unreadMessageIds.delete(id);
  }
  dmUnreadCounts.update((counts) => {
    const next = new Map(counts);
    next.delete(dmId);
    return next;
  });
}

function incrementDmUnread(dmId: string): void {
  dmUnreadCounts.update((counts) => {
    const next = new Map(counts);
    next.set(dmId, (next.get(dmId) ?? 0) + 1);
    return next;
  });
}

function bubbleDmChannel(dmId: string): void {
  dmChannels.update((list) => {
    const idx = list.findIndex((d) => d.id === dmId);
    if (idx <= 0) return list;
    const updated = [...list];
    const [dm] = updated.splice(idx, 1);
    return [dm, ...updated];
  });
}

/**
 * Fetch all DM channels for current user
 */
export async function fetchDmChannels(): Promise<void> {
  const session = currentSession();
  const list = await api.get<DmChannel[]>("/users/@me/channels");
  if (currentSession() === session) dmChannels.set(list);
}

/**
 * Open or create a DM with a recipient on this server.
 * @param recipientId - Local user snowflake ID
 */
export async function openDm(recipientId: string): Promise<DmChannel> {
  const session = currentSession();
  const dm = await api.post<DmChannel>("/users/@me/channels", {
    recipient_id: recipientId,
  });
  if (currentSession() !== session) return dm;
  dmChannels.update((list) => {
    const exists = list.find((d) => d.id === dm.id);
    return exists ? list : [dm, ...list];
  });
  return dm;
}

/**
 * Open or create a cross-server (federated) DM with a user on another server.
 * @param address - Remote user address in "username@hostname" format
 */
export async function openFederatedDm(address: string): Promise<DmChannel> {
  const session = currentSession();
  const dm = await api.post<DmChannel>("/users/@me/channels", {
    recipient_address: address,
  });
  if (currentSession() !== session) return dm;
  dmChannels.update((list) => {
    const exists = list.find((d) => d.id === dm.id);
    return exists ? list : [dm, ...list];
  });
  return dm;
}

/**
 * Fetch messages from a DM channel with pagination
 * @param dmId - DM channel ID
 * @param before - Message ID cursor for pagination
 */
export async function fetchDmMessages(
  dmId: string,
  before?: string,
): Promise<void> {
  const session = currentSession();
  if (activeDmId !== dmId) activeDmMessages.set([]);
  activeDmId = dmId;
  const request = ++messageRequest;
  clearDmUnread(dmId);
  dmLoading.set(true);
  const params = new URLSearchParams();
  if (before) params.set("before", before);
  params.set("limit", "50");
  try {
    const list = await api.get<DmMessage[]>(
      `/channels/@dms/${dmId}/messages?${params}`,
    );
    if (currentSession() !== session || request !== messageRequest) return;
    for (const message of list) rememberMessage(message);
    // Keep messages received while history was in flight, including overlaps.
    activeDmMessages.update((existing) => uniqueMessages([...list, ...existing]));
  } finally {
    if (currentSession() === session && request === messageRequest) {
      dmLoading.set(false);
    }
  }
}

/**
 * Send a message in a DM channel
 * @param dmId - DM channel ID
 * @param content - Message content
 */
export async function sendDmMessage(
  dmId: string,
  content: string,
): Promise<void> {
  const session = currentSession();
  const msg = await api.post<DmMessage>(`/channels/@dms/${dmId}/messages`, {
    content,
  });
  if (currentSession() !== session) return;
  rememberMessage(msg);
  // A gateway echo can arrive after navigation but before the POST response.
  // Once identified as our send, remove only that echo's unread contribution.
  if (unreadMessageIds.delete(msg.id)) {
    dmUnreadCounts.update((counts) => {
      const next = new Map(counts);
      const count = (next.get(msg.dm_id) ?? 0) - 1;
      if (count > 0) next.set(msg.dm_id, count);
      else next.delete(msg.dm_id);
      return next;
    });
  }
  if (activeDmId === dmId) {
    activeDmMessages.update((list) => uniqueMessages([...list, msg]));
  }
  bubbleDmChannel(dmId);
}

function handleDmMessage(data: unknown): void {
  currentSession();
  if (sessionKey === null) return;
  const { message } = data as { message: DmMessage };
  if (!rememberMessage(message)) return;
  if (message.dm_id === activeDmId) {
    activeDmMessages.update((list) => uniqueMessages([...list, message]));
  } else {
    unreadMessageIds.set(message.id, message.dm_id);
    incrementDmUnread(message.dm_id);
  }
  bubbleDmChannel(message.dm_id);
}

/**
 * Initialize WebSocket listener for DM message events
 * Updates active DM messages and bubbles DM to top of list
 */
export function initDmListener(): () => void {
  currentSession();
  if (listenerOwners++ === 0) gateway.on("DmMessageCreate", handleDmMessage);
  let released = false;
  return () => {
    if (released) return;
    released = true;
    if (--listenerOwners === 0) {
      gateway.off("DmMessageCreate", handleDmMessage);
      clearActiveDm();
    }
  };
}
