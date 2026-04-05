/**
 * @file Members store — manages member list for current server
 * @purpose Fetch and cache server members
 * @depends api/client, api/types
 */
import { writable } from "svelte/store";
import api from "$lib/api/client";
import { gateway } from "$lib/api/websocket";
import type { Member } from "$lib/api/types";

let activeSpaceId: string | null = null;

export const members = writable<Member[]>([]);
export const membersLoading = writable(false);

export async function fetchMembers(spaceId: string): Promise<void> {
  activeSpaceId = spaceId;
  membersLoading.set(true);
  try {
    const list = await api.get<Member[]>(`/servers/${spaceId}/members`);
    members.set(list);
  } finally {
    membersLoading.set(false);
  }
}

export function initMemberListeners(): void {
  gateway.on("MemberUpdate", (data: unknown) => {
    const event = data as { server_id: string };
    if (event.server_id === activeSpaceId && activeSpaceId) {
      fetchMembers(activeSpaceId).catch(() => {});
    }
  });
}
