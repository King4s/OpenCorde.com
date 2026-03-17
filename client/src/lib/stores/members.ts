/**
 * @file Members store — manages member list for current server
 * @purpose Fetch and cache server members
 * @depends api/client, api/types
 */
import { writable } from 'svelte/store';
import api from '$lib/api/client';
import type { Member } from '$lib/api/types';

export const members = writable<Member[]>([]);
export const membersLoading = writable(false);

export async function fetchMembers(serverId: string): Promise<void> {
  membersLoading.set(true);
  try {
    const list = await api.get<Member[]>(`/servers/${serverId}/members`);
    members.set(list);
  } finally {
    membersLoading.set(false);
  }
}
