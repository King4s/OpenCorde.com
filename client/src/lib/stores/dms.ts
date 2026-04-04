/**
 * @file DM store — direct message channels + messages
 * @purpose Manage DM channel list and active DM messages
 * @depends api/client, api/websocket, api/types
 */
import { writable, derived } from 'svelte/store';
import api from '$lib/api/client';
import { gateway } from '$lib/api/websocket';
import type { DmChannel, DmMessage } from '$lib/api/types';

export const dmChannels = writable<DmChannel[]>([]);
export const activeDmMessages = writable<DmMessage[]>([]);
export const dmLoading = writable(false);
export const dmUnreadCounts = writable<Map<string, number>>(new Map());
export const hasAnyDmUnread = derived(dmUnreadCounts, $counts => Array.from($counts.values()).some(c => c > 0));

let activeDmId: string | null = null;

function clearDmUnread(dmId: string): void {
	dmUnreadCounts.update(counts => {
		const next = new Map(counts);
		next.delete(dmId);
		return next;
	});
}

function incrementDmUnread(dmId: string): void {
	dmUnreadCounts.update(counts => {
		const next = new Map(counts);
		next.set(dmId, (next.get(dmId) ?? 0) + 1);
		return next;
	});
}

/**
 * Fetch all DM channels for current user
 */
export async function fetchDmChannels(): Promise<void> {
	const list = await api.get<DmChannel[]>('/users/@me/channels');
	dmChannels.set(list);
}

/**
 * Open or create a DM with a recipient on this server.
 * @param recipientId - Local user snowflake ID
 */
export async function openDm(recipientId: string): Promise<DmChannel> {
	const dm = await api.post<DmChannel>('/users/@me/channels', { recipient_id: recipientId });
	dmChannels.update(list => {
		const exists = list.find(d => d.id === dm.id);
		return exists ? list : [dm, ...list];
	});
	return dm;
}

/**
 * Open or create a cross-server (federated) DM with a user on another server.
 * @param address - Remote user address in "username@hostname" format
 */
export async function openFederatedDm(address: string): Promise<DmChannel> {
	const dm = await api.post<DmChannel>('/users/@me/channels', { recipient_address: address });
	dmChannels.update(list => {
		const exists = list.find(d => d.id === dm.id);
		return exists ? list : [dm, ...list];
	});
	return dm;
}

/**
 * Fetch messages from a DM channel with pagination
 * @param dmId - DM channel ID
 * @param before - Message ID cursor for pagination
 */
export async function fetchDmMessages(dmId: string, before?: string): Promise<void> {
	activeDmId = dmId;
	dmLoading.set(true);
	const params = new URLSearchParams();
	if (before) params.set('before', before);
	params.set('limit', '50');
	const list = await api.get<DmMessage[]>(`/channels/@dms/${dmId}/messages?${params}`);
	if (before) {
		activeDmMessages.update(existing => [...list, ...existing]);
	} else {
		activeDmMessages.set(list);
		clearDmUnread(dmId);
	}
	dmLoading.set(false);
}

/**
 * Send a message in a DM channel
 * @param dmId - DM channel ID
 * @param content - Message content
 */
export async function sendDmMessage(dmId: string, content: string): Promise<void> {
	const msg = await api.post<DmMessage>(`/channels/@dms/${dmId}/messages`, { content });
	activeDmMessages.update(list => [...list, msg]);
}

/**
 * Initialize WebSocket listener for DM message events
 * Updates active DM messages and bubbles DM to top of list
 */
export function initDmListener(): void {
	gateway.on('DmMessageCreate', (data: unknown) => {
		const evt = data as { message: DmMessage };
		if (evt.message.dm_id === activeDmId) {
			activeDmMessages.update(list => [...list, evt.message]);
		} else {
			incrementDmUnread(evt.message.dm_id);
		}
		// Update DM channel list to bubble up the DM with new message
		dmChannels.update(list => {
			const idx = list.findIndex(d => d.id === evt.message.dm_id);
			if (idx === -1) return list;
			const updated = [...list];
			const [dm] = updated.splice(idx, 1);
			return [dm, ...updated];
		});
	});
}
