<script lang="ts">
  /**
   * @file RoleManager.svelte — Server role manager modal
   * @purpose List roles, create/delete, and edit Display + Permissions + Members per role
   * @depends roles store, members store, RolePermissionsTab, permissions utils
   */
  import { roles, fetchRoles, createRole, updateRole, deleteRole } from '$lib/stores/roles';
  import type { Role } from '$lib/stores/roles';
  import { members, fetchMembers } from '$lib/stores/members';
  import { numberToPermissions, permissionsToNumber } from '$lib/utils/permissions';
  import RolePermissionsTab from './RolePermissionsTab.svelte';

  interface Props {
    spaceId: string;
    onClose: () => void;
  }

  let { spaceId, onClose }: Props = $props();

  // ── state ──────────────────────────────────────────────────────────────
  let newRoleName  = $state('');
  let newRoleColor = $state('#e5e7eb');
  let selected     = $state<Role | null>(null);
  let activeTab    = $state<'display' | 'permissions' | 'members'>('display');
  let error        = $state('');
  let saving       = $state(false);

  // edit fields (synced from selected role)
  let editName        = $state('');
  let editColor       = $state('#e5e7eb');
  let editMentionable = $state(false);
  let editPermissions = $state(0n);

  // ── lifecycle ─────────────────────────────────────────────────────────
  $effect(() => { fetchRoles(spaceId).catch(() => {}); });
  $effect(() => { fetchMembers(spaceId).catch(() => {}); });

  // ── helpers ───────────────────────────────────────────────────────────
  function hexToInt(hex: string): number {
    return parseInt(hex.replace('#', ''), 16);
  }
  function intToHex(n: number | null): string {
    if (n === null || n === undefined) return '#e5e7eb';
    return '#' + n.toString(16).padStart(6, '0');
  }

  function selectRole(role: Role) {
    selected        = role;
    activeTab       = 'display';
    editName        = role.name;
    editColor       = intToHex(role.color);
    editMentionable = role.mentionable;
    editPermissions = numberToPermissions(role.permissions);
    error           = '';
  }

  function membersWithRole(roleId: string) {
    return $members.filter((m) => m.role_ids?.includes(roleId));
  }

  // ── actions ───────────────────────────────────────────────────────────
  async function handleCreate() {
    if (!newRoleName.trim()) return;
    saving = true; error = '';
    try {
      const role = await createRole(spaceId, newRoleName.trim(), hexToInt(newRoleColor));
      newRoleName = '';
      selectRole(role);
    } catch (e: any) { error = e.message ?? 'Failed to create role'; }
    saving = false;
  }

  async function handleSave() {
    if (!selected || !editName.trim()) return;
    saving = true; error = '';
    try {
      await updateRole(spaceId, selected.id, {
        name:        editName.trim(),
        color:       hexToInt(editColor),
        mentionable: editMentionable,
        permissions: permissionsToNumber(editPermissions),
      });
      // refresh selected from store
      const updated = $roles.find((r) => r.id === selected!.id);
      if (updated) selected = updated;
    } catch (e: any) { error = e.message ?? 'Failed to save role'; }
    saving = false;
  }

  async function handleDelete(role: Role) {
    if (!confirm(`Delete role "${role.name}"?`)) return;
    error = '';
    try {
      await deleteRole(spaceId, role.id);
      if (selected?.id === role.id) selected = null;
    } catch (e: any) { error = e.message ?? 'Failed to delete role'; }
  }
</script>

<!-- ── Modal shell ─────────────────────────────────────────────────────── -->
<div
  class="fixed inset-0 z-50 bg-black/60 flex items-center justify-center p-4"
  role="dialog"
  aria-label="Manage roles"
>
  <div class="bg-gray-800 border border-gray-700 rounded-xl shadow-2xl w-full max-w-3xl flex flex-col max-h-[90vh]">

    <!-- Header -->
    <div class="flex items-center justify-between px-4 py-3 border-b border-gray-700 flex-shrink-0">
      <h2 class="font-semibold text-white text-sm">Manage Roles</h2>
      <button onclick={onClose} class="text-gray-400 hover:text-white text-lg leading-none">&#x2715;</button>
    </div>

    {#if error}
      <p class="px-4 py-2 text-gray-400 text-xs flex-shrink-0">{error}</p>
    {/if}

    <div class="flex flex-1 overflow-hidden">

      <!-- ── Left: role list ──────────────────────────────────────────── -->
      <div class="w-52 border-r border-gray-700 flex flex-col flex-shrink-0">

        <!-- Create row -->
        <div class="p-2 border-b border-gray-700">
          <div class="flex gap-1.5">
            <input
              bind:value={newRoleName}
              placeholder="New role"
              maxlength="100"
              class="flex-1 min-w-0 px-2 py-1 bg-gray-900 border border-gray-700 rounded text-white text-xs
                     focus:outline-none focus:border-gray-500"
              onkeydown={(e) => e.key === 'Enter' && handleCreate()}
            />
            <input
              type="color"
              bind:value={newRoleColor}
              class="w-7 h-7 rounded cursor-pointer border-0 bg-transparent flex-shrink-0"
              title="Role color"
            />
            <button
              onclick={handleCreate}
              disabled={saving || !newRoleName.trim()}
              class="px-2 py-1 bg-gray-600 hover:bg-gray-700 disabled:opacity-50
                     text-white text-xs rounded transition-colors flex-shrink-0"
            >+</button>
          </div>
        </div>

        <!-- Role list -->
        <div class="flex-1 overflow-y-auto p-1">
          {#each $roles as role (role.id)}
            <div
              class="flex items-center gap-2 px-2 py-1.5 rounded cursor-pointer group
                     {selected?.id === role.id ? 'bg-gray-600/30' : 'hover:bg-gray-700/50'}"
              onclick={() => selectRole(role)}
              role="button"
              tabindex="0"
              onkeydown={(e) => e.key === 'Enter' && selectRole(role)}
            >
              <span
                class="w-2.5 h-2.5 rounded-full flex-shrink-0"
                style="background-color: {intToHex(role.color)}"
              ></span>
              <span class="text-gray-200 text-xs flex-1 truncate">{role.name}</span>
              <button
                onclick={(e) => { e.stopPropagation(); handleDelete(role); }}
                class="opacity-0 group-hover:opacity-100 text-gray-400 hover:text-gray-300 text-xs
                       px-1 rounded hover:bg-gray-600 transition-opacity"
                title="Delete role"
              >&#x2715;</button>
            </div>
          {:else}
            <p class="text-gray-500 text-xs px-2 py-3">No roles yet.</p>
          {/each}
        </div>
      </div>

      <!-- ── Right: detail panel ─────────────────────────────────────── -->
      <div class="flex-1 flex flex-col overflow-hidden">
        {#if selected}

          <!-- Tab bar -->
          <div class="flex gap-1 px-4 pt-3 pb-0 border-b border-gray-700 flex-shrink-0">
            {#each (['display', 'permissions', 'members'] as const) as tab}
              <button
                onclick={() => (activeTab = tab)}
                class="px-3 py-1.5 text-xs rounded-t capitalize transition-colors
                       {activeTab === tab
                         ? 'bg-gray-700 text-white'
                         : 'text-gray-400 hover:text-gray-200'}"
              >{tab}</button>
            {/each}
          </div>

          <!-- Tab content -->
          <div class="flex-1 overflow-y-auto p-4">

            <!-- Display tab -->
            {#if activeTab === 'display'}
              <div class="space-y-4 max-w-sm">
                <div>
                  <label for="edit-role-name" class="block text-xs text-gray-400 mb-1">Role Name</label>
                  <input
                    id="edit-role-name"
                    bind:value={editName}
                    maxlength="100"
                    class="w-full px-2 py-1.5 bg-gray-900 border border-gray-700 rounded text-white text-sm
                           focus:outline-none focus:border-gray-500"
                  />
                </div>
                <div>
                  <label for="edit-role-color" class="block text-xs text-gray-400 mb-1">Role Color</label>
                  <div class="flex items-center gap-3">
                    <input
                      id="edit-role-color"
                      type="color"
                      bind:value={editColor}
                      class="w-10 h-10 rounded cursor-pointer border border-gray-600 bg-transparent"
                    />
                    <span class="text-gray-400 text-sm font-mono">{editColor}</span>
                  </div>
                </div>
                <div>
                  <label class="flex items-center gap-2 cursor-pointer">
                    <input
                      type="checkbox"
                      bind:checked={editMentionable}
                      class="w-4 h-4 accent-gray-500"
                    />
                    <span class="text-sm text-gray-200">Mentionable</span>
                  </label>
                  <p class="text-xs text-gray-500 mt-0.5 ml-6">Allow anyone to @mention this role</p>
                </div>
              </div>

            <!-- Permissions tab -->
            {:else if activeTab === 'permissions'}
              <RolePermissionsTab
                permissions={editPermissions}
                onChange={(p) => (editPermissions = p)}
              />

            <!-- Members tab -->
            {:else if activeTab === 'members'}
              {@const roleMembers = membersWithRole(selected.id)}
              {#if roleMembers.length === 0}
                <p class="text-gray-500 text-sm">No members have this role.</p>
              {:else}
                <div class="space-y-1">
                  {#each roleMembers as m (m.user_id)}
                    <div class="flex items-center gap-2 px-2 py-1.5 rounded hover:bg-gray-700/40">
                      <div class="w-6 h-6 rounded-full bg-gray-600 flex items-center justify-center
                                  text-white text-xs font-bold flex-shrink-0">
                        {(m.nickname ?? m.username)[0].toUpperCase()}
                      </div>
                      <span class="text-gray-200 text-sm">
                        {m.nickname ?? m.username}
                      </span>
                    </div>
                  {/each}
                </div>
              {/if}
            {/if}

          </div>

          <!-- Save button (not shown on members tab) -->
          {#if activeTab !== 'members'}
            <div class="flex justify-end gap-2 px-4 py-3 border-t border-gray-700 flex-shrink-0">
              <button
                onclick={() => (selected = null)}
                class="px-3 py-1.5 text-xs text-gray-400 hover:text-white rounded
                       hover:bg-gray-700 transition-colors"
              >Cancel</button>
              <button
                onclick={handleSave}
                disabled={saving || !editName.trim()}
                class="px-4 py-1.5 bg-gray-600 hover:bg-gray-700 disabled:opacity-50
                       text-white text-xs rounded transition-colors"
              >{saving ? 'Saving…' : 'Save Changes'}</button>
            </div>
          {/if}

        {:else}
          <div class="flex-1 flex items-center justify-center">
            <p class="text-gray-500 text-sm">Select a role to edit it.</p>
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>
