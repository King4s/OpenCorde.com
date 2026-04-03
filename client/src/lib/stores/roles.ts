/**
 * @file Roles store — manage server roles
 * @purpose Fetch, create, update, delete roles; assign/unassign to members
 */
import { writable } from 'svelte/store';
import api from '$lib/api/client';
import { gateway } from '$lib/api/websocket';

let activeServerId: string | null = null;

export interface Role {
  id: string;
  server_id: string;
  name: string;
  color: number | null;
  permissions: number;
  position: number;
  mentionable: boolean;
  created_at: string;
}

export const roles = writable<Role[]>([]);

export async function fetchRoles(serverId: string): Promise<void> {
  activeServerId = serverId;
  const list = await api.get<Role[]>(`/servers/${serverId}/roles`);
  roles.set(list);
}

export function initRoleListeners(): void {
  gateway.on('RoleCreate', (data: unknown) => {
    const event = data as { server_id: string; role: Role };
    if (event.server_id === activeServerId) {
      roles.update(list => [...list, event.role]);
    }
  });

  gateway.on('RoleUpdate', (data: unknown) => {
    const event = data as { server_id: string; role: Role };
    if (event.server_id === activeServerId) {
      roles.update(list => list.map(r => r.id === event.role.id ? event.role : r));
    }
  });

  gateway.on('RoleDelete', (data: unknown) => {
    const event = data as { server_id: string; role_id: string };
    if (event.server_id === activeServerId) {
      roles.update(list => list.filter(r => r.id !== event.role_id));
    }
  });
}

export async function createRole(serverId: string, name: string, color?: number): Promise<Role> {
  const role = await api.post<Role>(`/servers/${serverId}/roles`, {
    name,
    color: color ?? null,
    permissions: 0
  });
  roles.update(list => [...list, role]);
  return role;
}

export async function updateRole(
  serverId: string,
  roleId: string,
  data: Partial<{ name: string; color: number | null; permissions: number; mentionable: boolean }>
): Promise<void> {
  const updated = await api.patch<Role>(`/servers/${serverId}/roles/${roleId}`, data);
  roles.update(list => list.map(r => r.id === roleId ? updated : r));
}

export async function deleteRole(serverId: string, roleId: string): Promise<void> {
  await api.delete(`/servers/${serverId}/roles/${roleId}`);
  roles.update(list => list.filter(r => r.id !== roleId));
}

export async function assignRole(serverId: string, userId: string, roleId: string): Promise<void> {
  await api.put(`/servers/${serverId}/members/${userId}/roles/${roleId}`);
}

export async function unassignRole(serverId: string, userId: string, roleId: string): Promise<void> {
  await api.delete(`/servers/${serverId}/members/${userId}/roles/${roleId}`);
}
