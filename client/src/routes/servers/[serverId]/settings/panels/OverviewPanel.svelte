<!--
  @component OverviewPanel
  @purpose Server overview settings: name, description, icon display, danger zone
  @section settings/panels
-->
<script lang="ts">
  import { currentServer, fetchServers } from '$lib/stores/servers';
  import api from '$lib/api/client';

  let { serverId }: { serverId: string } = $props();

  let name = $state('');
  let description = $state('');
  let saving = $state(false);
  let deleting = $state(false);
  let error = $state('');
  let success = $state('');

  $effect(() => {
    if ($currentServer) {
      name = $currentServer.name;
      description = $currentServer.description ?? '';
    }
  });

  async function handleSave() {
    if (!name.trim() || !serverId) return;
    saving = true;
    error = '';
    success = '';
    try {
      await api.patch(`/servers/${serverId}`, {
        name: name.trim(),
        description: description.trim() || null
      });
      await fetchServers();
      success = 'Settings saved.';
    } catch (e: any) {
      error = e.message ?? 'Failed to save settings';
    } finally {
      saving = false;
    }
  }

  async function handleDelete() {
    if (!serverId) return;
    if (!confirm(`Delete server "${$currentServer?.name}"? This cannot be undone.`)) return;
    deleting = true;
    error = '';
    try {
      await api.delete(`/servers/${serverId}`);
      window.location.href = '/servers';
    } catch (e: any) {
      error = e.message ?? 'Failed to delete server';
      deleting = false;
    }
  }

  function getInitial(): string {
    return ($currentServer?.name ?? '?').charAt(0).toUpperCase();
  }
</script>

<div class="p-8 max-w-lg">
  <h1 class="text-xl font-semibold text-white mb-6">Overview</h1>

  {#if error}
    <div class="mb-4 px-3 py-2 bg-red-900/40 border border-red-700/50 rounded text-red-300 text-sm">{error}</div>
  {/if}
  {#if success}
    <div class="mb-4 px-3 py-2 bg-green-900/40 border border-green-700/50 rounded text-green-300 text-sm">{success}</div>
  {/if}

  <!-- Server Icon -->
  <div class="mb-6 flex items-center gap-4">
    {#if $currentServer?.icon_url}
      <img src={$currentServer.icon_url} alt="Server icon" class="w-20 h-20 rounded-full object-cover" />
    {:else}
      <div class="w-20 h-20 rounded-full bg-indigo-600 flex items-center justify-center text-white text-2xl font-bold select-none">
        {getInitial()}
      </div>
    {/if}
    <div>
      <p class="text-white text-sm font-medium">{$currentServer?.name ?? ''}</p>
      <p class="text-gray-400 text-xs mt-0.5">Server icon cannot be changed yet</p>
    </div>
  </div>

  <!-- Name & Description -->
  <div class="space-y-4 mb-8">
    <div>
      <label class="block text-xs font-semibold text-gray-400 uppercase mb-1" for="server-name">
        Server Name
      </label>
      <input
        id="server-name"
        type="text"
        bind:value={name}
        maxlength="100"
        class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm placeholder-gray-500 focus:outline-none focus:border-indigo-500"
      />
    </div>
    <div>
      <label class="block text-xs font-semibold text-gray-400 uppercase mb-1" for="server-desc">
        Description
      </label>
      <textarea
        id="server-desc"
        bind:value={description}
        rows="3"
        maxlength="500"
        placeholder="What's this server about?"
        class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm placeholder-gray-500 focus:outline-none focus:border-indigo-500 resize-none"
      ></textarea>
    </div>
    <button
      onclick={handleSave}
      disabled={saving || !name.trim()}
      class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white text-sm font-medium rounded transition-colors"
    >
      {saving ? 'Saving...' : 'Save Changes'}
    </button>
  </div>

  <!-- Danger Zone -->
  <div class="border-t border-gray-700 pt-6">
    <h2 class="text-sm font-semibold text-red-400 uppercase mb-2">Danger Zone</h2>
    <p class="text-gray-400 text-sm mb-3">Deleting the server is permanent and cannot be undone.</p>
    <button
      onclick={handleDelete}
      disabled={deleting}
      class="px-4 py-2 bg-red-700 hover:bg-red-600 disabled:opacity-50 text-white text-sm font-medium rounded transition-colors"
    >
      {deleting ? 'Deleting...' : 'Delete Server'}
    </button>
  </div>
</div>
