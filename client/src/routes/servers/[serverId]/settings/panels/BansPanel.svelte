<!--
  @component BansPanel
  @purpose List server bans with unban action
  @section settings/panels
-->
<script lang="ts">
  import api from '$lib/api/client';

  interface Ban {
    server_id: string;
    user_id: string;
    reason: string | null;
  }

  let { serverId }: { serverId: string } = $props();

  let bans = $state<Ban[]>([]);
  let loading = $state(false);
  let error = $state('');

  $effect(() => {
    if (serverId) loadBans();
  });

  async function loadBans() {
    loading = true;
    error = '';
    try {
      bans = await api.get<Ban[]>(`/servers/${serverId}/bans`);
    } catch (e: any) {
      error = e.message ?? 'Failed to load bans';
    } finally {
      loading = false;
    }
  }

  async function unban(userId: string) {
    try {
      await api.delete(`/servers/${serverId}/bans/${userId}`);
      bans = bans.filter(b => b.user_id !== userId);
    } catch (e: any) {
      error = e.message ?? 'Failed to unban user';
    }
  }
</script>

<div class="p-8 max-w-lg">
  <h1 class="text-xl font-semibold text-white mb-6">Bans</h1>

  {#if error}
    <div class="mb-4 px-3 py-2 bg-red-900/40 border border-red-700/50 rounded text-red-300 text-sm">{error}</div>
  {/if}

  {#if loading}
    <p class="text-gray-400 text-sm">Loading bans...</p>
  {:else if bans.length === 0}
    <div class="text-center py-12">
      <p class="text-gray-500 text-sm">No bans — this server has a clean record.</p>
    </div>
  {:else}
    <div class="space-y-2">
      {#each bans as ban (ban.user_id)}
        <div class="flex items-center gap-3 px-4 py-3 bg-gray-900 border border-gray-700 rounded">
          <div class="flex-1 min-w-0">
            <p class="text-gray-200 text-sm font-medium">User ID: <span class="font-mono text-indigo-400">{ban.user_id}</span></p>
            {#if ban.reason}
              <p class="text-gray-500 text-xs mt-0.5">Reason: {ban.reason}</p>
            {:else}
              <p class="text-gray-600 text-xs mt-0.5">No reason given</p>
            {/if}
          </div>
          <button
            onclick={() => unban(ban.user_id)}
            class="px-3 py-1 bg-gray-700 hover:bg-gray-600 text-white text-xs rounded transition-colors flex-shrink-0"
          >
            Unban
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>
