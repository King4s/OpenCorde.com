<!--
  @component ModerationPanel
  @purpose Server moderation settings: verification level, content filter
  @section settings/panels
-->
<script lang="ts">
  import api from '$lib/api/client';

  let { serverId }: { serverId: string } = $props();

  let verificationLevel = $state('0');
  let contentFilter = $state('0');
  let saving = $state(false);
  let success = $state('');
  let error = $state('');

  async function handleSave() {
    saving = true;
    success = '';
    error = '';
    try {
      await api.patch(`/servers/${serverId}`, {
        verification_level: parseInt(verificationLevel),
        content_filter: parseInt(contentFilter)
      });
      success = 'Moderation settings saved.';
    } catch (e: any) {
      error = e.message ?? 'Failed to save settings';
    } finally {
      saving = false;
    }
  }
</script>

<div class="p-8 max-w-lg">
  <h1 class="text-xl font-semibold text-white mb-6">Moderation</h1>

  {#if error}
    <div class="mb-4 px-3 py-2 bg-red-900/40 border border-red-700/50 rounded text-red-300 text-sm">{error}</div>
  {/if}
  {#if success}
    <div class="mb-4 px-3 py-2 bg-green-900/40 border border-green-700/50 rounded text-green-300 text-sm">{success}</div>
  {/if}

  <div class="space-y-6">
    <div>
      <label class="block text-xs font-semibold text-gray-400 uppercase mb-1" for="verification-level">
        Verification Level
      </label>
      <p class="text-gray-500 text-xs mb-2">Require members to meet criteria before sending messages.</p>
      <select
        id="verification-level"
        bind:value={verificationLevel}
        class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-indigo-500"
      >
        <option value="0">None — unrestricted</option>
        <option value="1">Low — verified email required</option>
        <option value="2">Medium — registered for 5+ minutes</option>
        <option value="3">High — member for 10+ minutes</option>
        <option value="4">Highest — verified phone required</option>
      </select>
    </div>

    <div>
      <label class="block text-xs font-semibold text-gray-400 uppercase mb-1" for="content-filter">
        Explicit Content Filter
      </label>
      <p class="text-gray-500 text-xs mb-2">Automatically scan and delete messages with explicit content.</p>
      <select
        id="content-filter"
        bind:value={contentFilter}
        class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-indigo-500"
      >
        <option value="0">Don't scan any messages</option>
        <option value="1">Scan messages from members without roles</option>
        <option value="2">Scan all messages</option>
      </select>
    </div>

    <button
      onclick={handleSave}
      disabled={saving}
      class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white text-sm font-medium rounded transition-colors"
    >
      {saving ? 'Saving...' : 'Save Changes'}
    </button>
  </div>
</div>
