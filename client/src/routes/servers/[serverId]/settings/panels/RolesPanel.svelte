<!--
  @component RolesPanel
  @purpose Inline role management panel — create, edit, delete server roles
  @section settings/panels
-->
<script lang="ts">
  import { roles, fetchRoles, createRole, updateRole, deleteRole } from '$lib/stores/roles';
  import type { Role } from '$lib/stores/roles';

  let { spaceId }: { spaceId: string } = $props();

  let newRoleName = $state('');
  let newRoleColor = $state('#e5e7eb');
  let editingRole = $state<Role | null>(null);
  let editName = $state('');
  let editColor = $state('#e5e7eb');
  let error = $state('');
  let loading = $state(false);

  $effect(() => {
    fetchRoles(spaceId).catch(() => {});
  });

  function hexToInt(hex: string): number {
    return parseInt(hex.replace('#', ''), 16);
  }

  function intToHex(n: number | null): string {
    if (n === null || n === undefined) return '#e5e7eb';
    return '#' + n.toString(16).padStart(6, '0');
  }

  async function handleCreate() {
    if (!newRoleName.trim()) return;
    loading = true;
    error = '';
    try {
      await createRole(spaceId, newRoleName.trim(), hexToInt(newRoleColor));
      newRoleName = '';
    } catch (e: any) {
      error = e.message ?? 'Failed to create role';
    }
    loading = false;
  }

  function startEdit(role: Role) {
    editingRole = role;
    editName = role.name;
    editColor = intToHex(role.color);
  }

  async function handleUpdate() {
    if (!editingRole || !editName.trim()) return;
    loading = true;
    error = '';
    try {
      await updateRole(spaceId, editingRole.id, {
        name: editName.trim(),
        color: hexToInt(editColor)
      });
      editingRole = null;
    } catch (e: any) {
      error = e.message ?? 'Failed to update role';
    }
    loading = false;
  }

  async function handleDelete(role: Role) {
    if (!confirm(`Delete role "${role.name}"?`)) return;
    error = '';
    try {
      await deleteRole(spaceId, role.id);
    } catch (e: any) {
      error = e.message ?? 'Failed to delete role';
    }
  }
</script>

<div class="w-full max-w-lg px-4 py-4 sm:p-8">
  <h1 class="text-xl font-semibold text-white mb-6">Roles</h1>

  {#if error}
    <p class="mb-3 text-gray-400 text-xs">{error}</p>
  {/if}

  <!-- Create Role -->
  <div class="mb-6 p-4 bg-gray-900 rounded border border-gray-700">
    <p class="text-xs text-gray-400 uppercase font-semibold mb-3">Create Role</p>
    <div class="flex gap-2">
      <input
        bind:value={newRoleName}
        placeholder="Role name"
        maxlength="100"
        class="flex-1 px-2 py-1.5 bg-gray-800 border border-gray-600 rounded text-white text-sm focus:outline-none focus:border-gray-500"
        onkeydown={(e) => e.key === 'Enter' && handleCreate()}
      />
      <input
        type="color"
        bind:value={newRoleColor}
        class="w-8 h-8 rounded cursor-pointer border-0 bg-transparent"
        title="Role color"
      />
      <button
        onclick={handleCreate}
        disabled={loading || !newRoleName.trim()}
        class="px-3 py-1.5 bg-gray-600 hover:bg-gray-700 disabled:opacity-50 text-white text-sm rounded transition-colors"
      >
        Add
      </button>
    </div>
  </div>

  <!-- Role List -->
  <div class="space-y-1">
    {#each $roles as role (role.id)}
      {#if editingRole?.id === role.id}
        <div class="flex gap-2 p-2 bg-gray-700/50 rounded">
          <input
            bind:value={editName}
            maxlength="100"
            class="flex-1 px-2 py-1 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-gray-500"
            onkeydown={(e) => e.key === 'Enter' && handleUpdate()}
          />
          <input
            type="color"
            bind:value={editColor}
            class="w-7 h-7 rounded cursor-pointer border-0 bg-transparent"
          />
          <button
            onclick={handleUpdate}
            class="px-2 py-1 bg-gray-600 hover:bg-gray-700 text-white text-xs rounded"
          >Save</button>
          <button
            onclick={() => (editingRole = null)}
            class="px-2 py-1 bg-gray-600 hover:bg-gray-500 text-white text-xs rounded"
          >Cancel</button>
        </div>
      {:else}
        <div class="flex items-center gap-2 px-3 py-2 rounded hover:bg-gray-700/50 group">
          <span
            class="w-3 h-3 rounded-full flex-shrink-0"
            style="background-color: {intToHex(role.color)}"
          ></span>
          <span class="text-gray-200 text-sm flex-1 truncate">{role.name}</span>
          <div class="opacity-0 group-hover:opacity-100 flex gap-1 transition-opacity">
            <button
              onclick={() => startEdit(role)}
              class="text-gray-400 hover:text-white text-xs px-1.5 py-0.5 rounded hover:bg-gray-600"
            >Edit</button>
            <button
              onclick={() => handleDelete(role)}
              class="text-gray-400 hover:text-gray-300 text-xs px-1.5 py-0.5 rounded hover:bg-gray-600"
            >Delete</button>
          </div>
        </div>
      {/if}
    {:else}
      <p class="text-gray-500 text-sm px-2 py-3">No roles yet. Create one above.</p>
    {/each}
  </div>
</div>
