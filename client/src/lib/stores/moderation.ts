/**
 * @file Moderation store — ban, kick, timeout operations
 * @purpose Server moderation actions (ban/unban, kick, timeout)
 * @depends api/client, svelte/store
 */

import { writable } from 'svelte/store';
import api from '$lib/api/client';

export interface Ban {
	server_id: string;
	user_id: string;
	reason: string | null;
}

/** Writable store for server ban list */
export const banList = writable<Ban[]>([]);

/**
 * Fetch all bans for a server and update store
 */
export async function fetchBans(serverId: string): Promise<void> {
	const bans = await api.get<Ban[]>(`/servers/${serverId}/bans`);
	banList.set(bans);
}

/**
 * Ban a user from a server
 * @param serverId Server ID
 * @param userId User ID to ban
 * @param reason Optional ban reason
 */
export async function banUser(serverId: string, userId: string, reason?: string): Promise<void> {
	await api.put(`/servers/${serverId}/bans/${userId}`, {
		reason: reason ?? null,
		delete_messages: false
	});
}

/**
 * Unban a user from a server
 */
export async function unbanUser(serverId: string, userId: string): Promise<void> {
	await api.delete(`/servers/${serverId}/bans/${userId}`);
	banList.update(list => list.filter(b => b.user_id !== userId));
}

/**
 * Kick a user from a server
 */
export async function kickUser(serverId: string, userId: string): Promise<void> {
	await api.delete(`/servers/${serverId}/members/${userId}`);
}

/**
 * Timeout (mute) a user in a server
 * @param serverId Server ID
 * @param userId User ID to timeout
 * @param durationSeconds Timeout duration in seconds
 * @param reason Optional timeout reason
 */
export async function timeoutUser(
	serverId: string,
	userId: string,
	durationSeconds: number,
	reason?: string
): Promise<void> {
	await api.put(`/servers/${serverId}/members/${userId}/timeout`, {
		duration_seconds: durationSeconds,
		reason: reason ?? null
	});
}
