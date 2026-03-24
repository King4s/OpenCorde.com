<!--
  @component AutomodPanel
  @purpose Inline automod keyword filter rule management (no modal wrapper)
  @section settings/panels
-->
<script lang="ts">
  import { automodStore } from '$lib/stores/automod.svelte';
  import { onMount } from 'svelte';
  import AutomodRuleItem from '$lib/components/modals/AutomodRuleItem.svelte';
  import AutomodRuleForm from '$lib/components/modals/AutomodRuleForm.svelte';

  let { serverId }: { serverId: string } = $props();

  let newName = $state('Keyword Filter');
  let newKeywords = $state('');
  let newAction = $state('delete');
  let creating = $state(false);
  let error = $state('');

  onMount(() => automodStore.fetch(serverId));

  async function handleCreate() {
    const keywords = newKeywords
      .split('\n')
      .map((k) => k.trim())
      .filter(Boolean);
    if (keywords.length === 0) {
      error = 'Add at least one keyword';
      return;
    }
    creating = true;
    error = '';
    try {
      await automodStore.create(serverId, keywords, newName, newAction);
      newKeywords = '';
      newName = 'Keyword Filter';
    } catch (e: unknown) {
      error = (e as { message?: string }).message ?? 'Failed to create rule';
    } finally {
      creating = false;
    }
  }

  async function toggleEnabled(id: string, enabled: boolean) {
    await automodStore.update(id, { enabled: !enabled });
  }
</script>

<div class="p-8 max-w-lg">
  <h1 class="text-xl font-semibold text-white mb-6">AutoMod</h1>

  <!-- Existing rules -->
  <div class="mb-6 space-y-2">
    {#each automodStore.rules as rule (rule.id)}
      <AutomodRuleItem
        id={rule.id}
        name={rule.name}
        keywords={rule.keywords}
        action={rule.action}
        enabled={rule.enabled}
        onToggle={() => toggleEnabled(rule.id, rule.enabled)}
        onDelete={() => automodStore.remove(rule.id)}
      />
    {/each}
    {#if automodStore.rules.length === 0}
      <p class="text-gray-500 text-sm">No rules yet — add one below.</p>
    {/if}
  </div>

  <!-- Add rule form -->
  <div class="bg-gray-900 rounded border border-gray-700 p-4">
    <AutomodRuleForm
      name={newName}
      keywords={newKeywords}
      action={newAction}
      error={error}
      creating={creating}
      onNameChange={(n) => (newName = n)}
      onKeywordsChange={(k) => (newKeywords = k)}
      onActionChange={(a) => (newAction = a)}
      onSubmit={handleCreate}
    />
  </div>
</div>
