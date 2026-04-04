<!--
  @component InvitesPanel
  @purpose List and revoke server invite links
  @section settings/panels
-->
<script lang="ts">
  import api from '$lib/api/client';

  interface Invite {
    code: string;
    uses: number;
    max_uses: number | null;
    expires_at: string | null;
    created_at: string;
  }

  let { spaceId }: { spaceId: string } = $props();

  let invites = $state<Invite[]>([]);
  let loading = $state(false);
  let error = $state('');

  $effect(() => {
    if (spaceId) loadInvites();
  });

  async function loadInvites() {
    loading = true;
    error = '';
    try {
      invites = await api.get<Invite[]>(`/servers/${spaceId}/invites`);
    } catch (e: any) {
      error = e.message ?? 'Failed to load invites';
      invites = [];
    } finally {
      loading = false;
    }
  }

  async function revokeInvite(code: string) {
    try {
      await api.delete(`/servers/${spaceId}/invites/${code}`);
      invites = invites.filter(i => i.code !== code);
    } catch (e: any) {
      error = e.message ?? 'Failed to revoke invite';
    }
  }

  async function createInvite() {
    try {
      const inv = await api.post<Invite>(`/servers/${spaceId}/invites`, {});
      invites = [inv, ...invites];
    } catch (e: any) {
      error = e.message ?? 'Failed to create invite';
    }
  }

  function formatExpiry(expiresAt: string | null): string {
    if (!expiresAt) return 'Never';
    return new Date(expiresAt).toLocaleDateString();
  }
</script>

<div class="p-8 max-w-lg">
  <div class="flex items-center justify-between mb-6">
    <h1 class="text-xl font-semibold text-white">Invites</h1>
    <button
      onclick={createInvite}
      class="px-3 py-1.5 bg-gray-600 hover:bg-gray-700 text-white text-sm font-medium rounded transition-colors"
    >
      Create Invite
    </button>
  </div>

  {#if error}
    <div class="mb-4 px-3 py-2 bg-gray-900/40 border border-gray-700/50 rounded text-gray-300 text-sm">{error}</div>
  {/if}

  {#if loading}
    <p class="text-gray-400 text-sm">Loading invites...</p>
  {:else if invites.length === 0}
    <div class="text-center py-12">
      <p class="text-gray-500 text-sm">No active invites.</p>
    </div>
  {:else}
    <div class="space-y-2">
      {#each invites as inv (inv.code)}
        <div class="flex items-center gap-3 px-4 py-3 bg-gray-900 border border-gray-700 rounded">
          <code class="text-gray-400 text-sm flex-1">{inv.code}</code>
          <span class="text-gray-500 text-xs whitespace-nowrap">{inv.uses} uses</span>
          <span class="text-gray-500 text-xs whitespace-nowrap">Expires: {formatExpiry(inv.expires_at)}</span>
          <button
            onclick={() => revokeInvite(inv.code)}
            class="px-2 py-1 text-xs text-gray-400 hover:bg-gray-800 rounded transition-colors flex-shrink-0"
          >Revoke</button>
        </div>
      {/each}
    </div>
  {/if}
</div>
