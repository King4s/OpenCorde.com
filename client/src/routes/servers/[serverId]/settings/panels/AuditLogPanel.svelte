<!--
  @component AuditLogPanel
  @purpose Inline audit log with pagination — server moderation history
  @section settings/panels
-->
<script lang="ts">
  import api from '$lib/api/client';

  interface AuditEntry {
    id: string;
    actor_id: string | null;
    actor_username: string | null;
    action: string;
    target_id: string | null;
    target_type: string | null;
    changes: Record<string, unknown> | null;
    created_at: string;
  }

  let { spaceId }: { spaceId: string } = $props();

  let entries = $state<AuditEntry[]>([]);
  let loading = $state(false);
  let error = $state('');

  $effect(() => {
    if (spaceId) loadEntries();
  });

  async function loadEntries() {
    loading = true;
    error = '';
    try {
      entries = await api.get<AuditEntry[]>(`/servers/${spaceId}/audit-log`);
    } catch (e: any) {
      error = e.message ?? 'Failed to load audit log';
      entries = [];
    } finally {
      loading = false;
    }
  }

  async function loadMore() {
    if (!spaceId || entries.length === 0) return;
    loading = true;
    try {
      const lastId = entries[entries.length - 1].id;
      const more = await api.get<AuditEntry[]>(
        `/servers/${spaceId}/audit-log?before=${lastId}&limit=50`
      );
      entries = [...entries, ...more];
    } catch (e: any) {
      error = e.message ?? 'Failed to load more';
    } finally {
      loading = false;
    }
  }

  function getActionColor(action: string): string {
    if (action.includes('ban')) return 'bg-gray-900/40 text-gray-300';
    if (action.includes('kick')) return 'bg-gray-900/40 text-gray-300';
    if (action.includes('timeout')) return 'bg-gray-900/40 text-gray-300';
    if (action.includes('create')) return 'bg-gray-900/40 text-gray-300';
    if (action.includes('delete')) return 'bg-gray-900/40 text-gray-300';
    if (action.includes('update')) return 'bg-gray-900/40 text-gray-300';
    return 'bg-gray-700 text-gray-300';
  }

  function formatDate(d: string): string {
    return new Date(d).toLocaleString();
  }

  function getActionLabel(action: string): string {
    return action.split('.').map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' ');
  }
</script>

<div class="p-8 max-w-4xl">
  <h1 class="text-xl font-semibold text-white mb-6">Audit Log</h1>

  {#if error}
    <div class="mb-4 px-3 py-2 bg-gray-900/40 border border-gray-700/50 rounded text-gray-300 text-sm">{error}</div>
  {/if}

  {#if loading && entries.length === 0}
    <p class="text-gray-400 text-sm">Loading audit log...</p>
  {:else if entries.length === 0}
    <p class="text-gray-400 text-sm">No audit log entries yet.</p>
  {:else}
    <div class="space-y-2">
      {#each entries as entry (entry.id)}
        <div class="px-4 py-3 bg-gray-900 border border-gray-700 rounded">
          <div class="flex items-start justify-between gap-4 mb-2">
            <div class="flex-1">
              <div class="flex items-center gap-2 mb-1">
                <span class="text-xs text-gray-400 font-mono">{entry.actor_username ?? 'Unknown'}</span>
                <span class="px-2 py-0.5 rounded text-xs font-medium {getActionColor(entry.action)}">
                  {getActionLabel(entry.action)}
                </span>
              </div>
              {#if entry.target_id && entry.target_type}
                <div class="text-xs text-gray-500">
                  {entry.target_type}: <span class="font-mono">{entry.target_id}</span>
                </div>
              {/if}
            </div>
            <time class="text-xs text-gray-500 whitespace-nowrap">{formatDate(entry.created_at)}</time>
          </div>
          {#if entry.changes}
            <div class="text-xs bg-gray-800 rounded p-2 font-mono text-gray-400 overflow-x-auto">
              <pre>{JSON.stringify(entry.changes, null, 2)}</pre>
            </div>
          {/if}
        </div>
      {/each}
    </div>

    {#if !loading}
      <button
        onclick={loadMore}
        class="mt-6 px-4 py-2 bg-gray-600 hover:bg-gray-700 text-white text-sm font-medium rounded transition-colors"
      >
        Load More
      </button>
    {:else}
      <p class="mt-6 text-gray-400 text-sm">Loading...</p>
    {/if}
  {/if}
</div>
