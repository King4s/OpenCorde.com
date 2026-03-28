/**
 * @file notificationSettings — per-channel notification level overrides
 * @purpose Load and update the user's channel notification preferences
 * @depends api/client
 */
import { writable, derived } from 'svelte/store';
import api from '$lib/api/client';

export type NotifLevel = 0 | 1 | 2; // 0=all, 1=mentions-only, 2=muted

interface ChannelNotifSetting {
	channel_id: string;
	level: NotifLevel;
}

// Map from channel_id → level (only stores explicit overrides)
export const notifSettings = writable<Map<string, NotifLevel>>(new Map());

/** Fetch all per-channel overrides for the current user. */
export async function fetchNotifSettings(): Promise<void> {
	try {
		const rows = await api.get<ChannelNotifSetting[]>('/users/@me/notification-settings');
		const map = new Map<string, NotifLevel>();
		for (const r of rows) map.set(r.channel_id, r.level as NotifLevel);
		notifSettings.set(map);
	} catch {
		// Non-fatal; defaults apply
	}
}

/** Set notification level for a channel (0=all, 1=mentions, 2=muted). */
export async function setNotifLevel(channelId: string, level: NotifLevel): Promise<void> {
	if (level === 0) {
		// Deleting the override restores the server default (level 0)
		await api.delete(`/channels/${channelId}/notification-settings`);
		notifSettings.update(m => { m.delete(channelId); return new Map(m); });
	} else {
		await api.put(`/channels/${channelId}/notification-settings`, { level });
		notifSettings.update(m => { m.set(channelId, level); return new Map(m); });
	}
}

/** Derive the effective level for a channel (falls back to 0=all). */
export const notifLevelFor = derived(notifSettings, $map => (channelId: string): NotifLevel => {
	return $map.get(channelId) ?? 0;
});

/** Labels for each notification level. */
export const NOTIF_LABELS: Record<NotifLevel, string> = {
	0: 'All Messages',
	1: 'Only Mentions',
	2: 'Muted',
};
