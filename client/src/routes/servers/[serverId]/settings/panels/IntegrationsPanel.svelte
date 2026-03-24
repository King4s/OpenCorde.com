<!--
  @component IntegrationsPanel
  @purpose Webhooks and slash commands inline integration management
  @section settings/panels
-->
<script lang="ts">
  import { slashCommandsStore } from '$lib/stores/slashCommands.svelte';
  import { onMount } from 'svelte';
  import SlashCommandForm from '$lib/components/modals/SlashCommandForm.svelte';
  import SlashCommandList from '$lib/components/modals/SlashCommandList.svelte';

  let { serverId }: { serverId: string } = $props();

  let newName = $state('');
  let newDescription = $state('');
  let newHandlerUrl = $state('');
  let creating = $state(false);
  let error = $state('');

  onMount(() => slashCommandsStore.fetchCommands(serverId));

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
</script>

<div class="p-8 max-w-lg">
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
</div>
