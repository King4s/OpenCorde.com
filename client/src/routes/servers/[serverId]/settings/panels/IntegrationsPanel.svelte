<!--
  @component IntegrationsPanel
  @purpose Webhooks, slash commands, and Discord bridge integration management
  @section settings/panels
-->
<script lang="ts">
  import { slashCommandsStore } from '$lib/stores/slashCommands.svelte';
  import { onMount } from 'svelte';
  import SlashCommandForm from '$lib/components/modals/SlashCommandForm.svelte';
  import SlashCommandList from '$lib/components/modals/SlashCommandList.svelte';
  import api from '$lib/api/client';

  let { serverId }: { serverId: string } = $props();

  // ── Slash Commands ────────────────────────────────────────────────────────
  let newName = $state('');
  let newDescription = $state('');
  let newHandlerUrl = $state('');
  let creating = $state(false);
  let error = $state('');

  onMount(() => {
    slashCommandsStore.fetchCommands(serverId);
    loadMappings();
  });

  async function handleCreate() {
    if (!newName.trim()) { error = 'Command name is required'; return; }
    if (!newHandlerUrl.trim()) { error = 'Handler URL is required'; return; }
    creating = true;
    error = '';
    try {
      await slashCommandsStore.createCommand(
        serverId,
        newName.trim(),
        newDescription.trim(),
        newHandlerUrl.trim()
      );
      newName = '';
      newDescription = '';
      newHandlerUrl = '';
    } catch (e: unknown) {
      error = (e as { message?: string }).message ?? 'Failed to create command';
    } finally {
      creating = false;
    }
  }

  async function handleDelete(id: string) {
    try {
      await slashCommandsStore.deleteCommand(id);
    } catch (e: unknown) {
      error = (e as { message?: string }).message ?? 'Failed to delete command';
    }
  }

  // ── Discord Bridge ────────────────────────────────────────────────────────
  interface BridgeMapping {
    id: number;
    discord_guild_id: string;
    discord_channel_id: string;
    discord_webhook_id: string | null;
    opencorde_channel_id: string;
    enabled: boolean;
    last_discord_msg_id: number;
    last_opencorde_msg_id: number;
    created_at: string;
  }

  let mappings = $state<BridgeMapping[]>([]);
  let bridgeError = $state('');
  let bridgeLoading = $state(false);

  // New mapping form fields
  let bDiscordGuildId = $state('');
  let bDiscordChannelId = $state('');
  let bWebhookId = $state('');
  let bWebhookToken = $state('');
  let bOpenCordeChannelId = $state('');
  let bCreating = $state(false);

  async function loadMappings() {
    bridgeLoading = true;
    bridgeError = '';
    try {
      mappings = await api.get<BridgeMapping[]>(`/servers/${serverId}/bridge/mappings`);
    } catch (e: unknown) {
      bridgeError = (e as { message?: string }).message ?? 'Failed to load mappings';
    } finally {
      bridgeLoading = false;
    }
  }

  async function createMapping() {
    if (!bDiscordGuildId.trim() || !bDiscordChannelId.trim() || !bOpenCordeChannelId.trim()) {
      bridgeError = 'Guild ID, Discord channel ID, and OpenCorde channel ID are required';
      return;
    }
    bCreating = true;
    bridgeError = '';
    try {
      const created = await api.post<BridgeMapping>(`/servers/${serverId}/bridge/mappings`, {
        discord_guild_id: bDiscordGuildId.trim(),
        discord_channel_id: bDiscordChannelId.trim(),
        discord_webhook_id: bWebhookId.trim() || null,
        discord_webhook_token: bWebhookToken.trim() || null,
        opencorde_channel_id: bOpenCordeChannelId.trim(),
      });
      mappings = [...mappings, created];
      bDiscordGuildId = '';
      bDiscordChannelId = '';
      bWebhookId = '';
      bWebhookToken = '';
      bOpenCordeChannelId = '';
    } catch (e: unknown) {
      bridgeError = (e as { message?: string }).message ?? 'Failed to create mapping';
    } finally {
      bCreating = false;
    }
  }

  async function toggleMapping(m: BridgeMapping) {
    try {
      const updated = await api.patch<BridgeMapping>(
        `/servers/${serverId}/bridge/mappings/${m.id}`,
        { enabled: !m.enabled }
      );
      mappings = mappings.map(x => x.id === m.id ? updated : x);
    } catch (e: unknown) {
      bridgeError = (e as { message?: string }).message ?? 'Failed to update mapping';
    }
  }

  async function removeMapping(id: number) {
    try {
      await api.delete(`/servers/${serverId}/bridge/mappings/${id}`);
      mappings = mappings.filter(m => m.id !== id);
    } catch (e: unknown) {
      bridgeError = (e as { message?: string }).message ?? 'Failed to delete mapping';
    }
  }
</script>

<div class="p-8 max-w-2xl space-y-10">

  <!-- ── Discord Bridge ───────────────────────────────────────────────── -->
  <section>
    <h1 class="text-xl font-semibold text-white mb-1">Discord Bridge</h1>
    <p class="text-sm text-gray-400 mb-4">
      Sync messages bidirectionally between Discord channels and OpenCorde channels.
      Requires <span class="text-gray-200 font-medium">DISCORD_TOKEN</span> set on the bridge service.
    </p>

    {#if bridgeError}
      <div class="mb-4 px-3 py-2 bg-red-900/40 border border-red-700/50 rounded text-red-300 text-sm">{bridgeError}</div>
    {/if}

    <!-- Existing mappings -->
    {#if bridgeLoading}
      <p class="text-gray-500 text-sm">Loading…</p>
    {:else if mappings.length === 0}
      <p class="text-gray-500 text-sm mb-4">No bridge mappings configured.</p>
    {:else}
      <div class="space-y-2 mb-6">
        {#each mappings as m (m.id)}
          <div class="flex items-center justify-between bg-gray-900 border border-gray-700 rounded px-4 py-3 text-sm">
            <div class="space-y-0.5 min-w-0">
              <div class="text-gray-200 font-mono text-xs">
                Discord <span class="text-indigo-400">#{m.discord_channel_id}</span>
                ↔ OpenCorde <span class="text-indigo-400">#{m.opencorde_channel_id}</span>
              </div>
              <div class="text-gray-500 text-xs">
                Guild {m.discord_guild_id}
                {#if m.discord_webhook_id}· webhook configured{/if}
              </div>
            </div>
            <div class="flex items-center gap-3 ml-4 flex-shrink-0">
              <button
                onclick={() => toggleMapping(m)}
                class="px-2 py-1 rounded text-xs font-medium transition-colors {m.enabled
                  ? 'bg-green-700/40 text-green-300 hover:bg-green-700/60'
                  : 'bg-gray-700 text-gray-400 hover:bg-gray-600'}"
              >
                {m.enabled ? 'Enabled' : 'Disabled'}
              </button>
              <button
                onclick={() => removeMapping(m.id)}
                class="text-red-400 hover:text-red-300 text-xs px-2 py-1 rounded hover:bg-red-900/30 transition-colors"
              >
                Remove
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}

    <!-- Add mapping form -->
    <div class="bg-gray-900 rounded border border-gray-700 p-4 space-y-3">
      <h3 class="text-sm font-semibold text-gray-300">Add Channel Bridge</h3>
      <div class="grid grid-cols-2 gap-3">
        <div>
          <label class="block text-xs text-gray-400 mb-1">Discord Guild ID</label>
          <input
            bind:value={bDiscordGuildId}
            type="text"
            placeholder="123456789012345678"
            class="w-full bg-gray-800 border border-gray-600 rounded px-3 py-1.5 text-sm text-gray-200 placeholder-gray-600 focus:outline-none focus:border-indigo-500"
          />
        </div>
        <div>
          <label class="block text-xs text-gray-400 mb-1">Discord Channel ID</label>
          <input
            bind:value={bDiscordChannelId}
            type="text"
            placeholder="123456789012345678"
            class="w-full bg-gray-800 border border-gray-600 rounded px-3 py-1.5 text-sm text-gray-200 placeholder-gray-600 focus:outline-none focus:border-indigo-500"
          />
        </div>
        <div>
          <label class="block text-xs text-gray-400 mb-1">OpenCorde Channel ID</label>
          <input
            bind:value={bOpenCordeChannelId}
            type="text"
            placeholder="Channel snowflake ID"
            class="w-full bg-gray-800 border border-gray-600 rounded px-3 py-1.5 text-sm text-gray-200 placeholder-gray-600 focus:outline-none focus:border-indigo-500"
          />
        </div>
        <div>
          <label class="block text-xs text-gray-400 mb-1">Webhook ID <span class="text-gray-600">(optional)</span></label>
          <input
            bind:value={bWebhookId}
            type="text"
            placeholder="For OpenCorde → Discord"
            class="w-full bg-gray-800 border border-gray-600 rounded px-3 py-1.5 text-sm text-gray-200 placeholder-gray-600 focus:outline-none focus:border-indigo-500"
          />
        </div>
        <div class="col-span-2">
          <label class="block text-xs text-gray-400 mb-1">Webhook Token <span class="text-gray-600">(optional)</span></label>
          <input
            bind:value={bWebhookToken}
            type="password"
            placeholder="Discord webhook token"
            class="w-full bg-gray-800 border border-gray-600 rounded px-3 py-1.5 text-sm text-gray-200 placeholder-gray-600 focus:outline-none focus:border-indigo-500"
          />
        </div>
      </div>
      <button
        onclick={createMapping}
        disabled={bCreating}
        class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white text-sm rounded transition-colors"
      >
        {bCreating ? 'Adding…' : 'Add Bridge'}
      </button>
    </div>
  </section>

  <!-- ── Slash Commands ───────────────────────────────────────────────── -->
  <section>
    <h1 class="text-xl font-semibold text-white mb-6">Slash Commands</h1>

    {#if error}
      <div class="mb-4 px-3 py-2 bg-red-900/40 border border-red-700/50 rounded text-red-300 text-sm">{error}</div>
    {/if}

    <div class="mb-6">
      <SlashCommandList
        commands={slashCommandsStore.commands}
        onDelete={handleDelete}
      />
    </div>

    <div class="bg-gray-900 rounded border border-gray-700 p-4">
      <SlashCommandForm
        name={newName}
        description={newDescription}
        handlerUrl={newHandlerUrl}
        error=""
        isLoading={creating}
        onNameChange={(v) => (newName = v)}
        onDescriptionChange={(v) => (newDescription = v)}
        onHandlerUrlChange={(v) => (newHandlerUrl = v)}
        onSubmit={handleCreate}
      />
    </div>
  </section>

</div>
