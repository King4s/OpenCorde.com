<!--
  @component AutomodManager
  @purpose Configure keyword filter rules for a server.
  @version 1.0.0
-->
<script lang="ts">
  import { automodStore } from '$lib/stores/automod.svelte';
  import { onMount } from 'svelte';
  import AutomodRuleItem from './AutomodRuleItem.svelte';
  import AutomodRuleForm from './AutomodRuleForm.svelte';

  let { serverId, onClose }: { serverId: string; onClose: () => void } = $props();

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
      error =
        (e as { message?: string }).message ?? 'Failed to create rule';
    } finally {
      creating = false;
    }
  }

  async function toggleEnabled(id: string, enabled: boolean) {
    await automodStore.update(id, { enabled: !enabled });
  }
</script>

<div
  class="backdrop"
  onclick={onClose}
  role="presentation"
  onkeydown={(e) => e.key === 'Escape' && onClose()}
>
  <div
    class="modal"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.key === 'Escape' && onClose()}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
  >
    <div class="header">
      <h2>AutoMod — Keyword Filter</h2>
      <button onclick={onClose}>✕</button>
    </div>
    <div class="body">
      <div class="rules-list">
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
          <p class="empty">No rules yet.</p>
        {/if}
      </div>

      <AutomodRuleForm
        name={newName}
        keywords={newKeywords}
        action={newAction}
        error={error}
        creating={creating}
        onNameChange={(n) => newName = n}
        onKeywordsChange={(k) => newKeywords = k}
        onActionChange={(a) => newAction = a}
        onSubmit={handleCreate}
      />
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: #2b2d31;
    border-radius: 8px;
    width: 520px;
    max-width: 90vw;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid #35373c;
  }

  .header h2 {
    margin: 0;
    font-size: 16px;
    color: #f2f3f5;
  }

  .header button {
    background: none;
    border: none;
    color: #b5bac1;
    cursor: pointer;
    font-size: 18px;
  }

  .body {
    overflow-y: auto;
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .rules-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .empty {
    color: #b5bac1;
    font-size: 13px;
    text-align: center;
    padding: 12px;
  }
</style>
