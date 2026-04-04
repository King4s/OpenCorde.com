<!--
  @component EmojisPanel
  @purpose Inline custom emoji management — upload, list, delete
  @section settings/panels
-->
<script lang="ts">
  import { emojiStore } from '$lib/stores/emojis';
  import { onMount } from 'svelte';
  import EmojiGrid from '$lib/components/modals/EmojiGrid.svelte';
  import EmojiUploadForm from '$lib/components/modals/EmojiUploadForm.svelte';

  let { spaceId }: { spaceId: string } = $props();

  let name = $state('');
  let file: File | null = $state(null);
  let uploading = $state(false);
  let error = $state('');

  onMount(() => emojiStore.fetchEmojis(spaceId));

  async function handleUpload() {
    if (!name.trim() || !file) { error = 'Please enter a name and select a file'; return; }
    if (name.length < 2 || name.length > 32) { error = 'Name must be 2-32 characters'; return; }
    if (!/^[a-z0-9_]+$/.test(name)) { error = 'Name can only contain lowercase letters, numbers, and underscores'; return; }
    uploading = true;
    error = '';
    try {
      await emojiStore.uploadEmoji(spaceId, name, file);
      name = '';
      file = null;
    } catch (e: unknown) {
      error = (e as { message?: string }).message ?? 'Failed to upload emoji';
    } finally {
      uploading = false;
    }
  }

  async function handleDelete(emojiId: string, emojiName: string) {
    if (!confirm(`Delete emoji "${emojiName}"?`)) return;
    try {
      await emojiStore.deleteEmoji(spaceId, emojiId);
    } catch (e: unknown) {
      error = (e as { message?: string }).message ?? 'Failed to delete emoji';
    }
  }
</script>

<div class="p-8 max-w-lg">
  <h1 class="text-xl font-semibold text-white mb-6">Custom Emojis</h1>

  {#if error}
    <div class="mb-4 px-3 py-2 bg-gray-900/40 border border-gray-700/50 rounded text-gray-300 text-sm">{error}</div>
  {/if}

  <div class="mb-6">
    <EmojiGrid
      emojis={$emojiStore.emojis}
      onDelete={handleDelete}
    />
  </div>

  <div class="bg-gray-900 rounded border border-gray-700 p-4">
    <p class="text-xs text-gray-400 uppercase font-semibold mb-3">Upload Emoji</p>
    <EmojiUploadForm
      name={name}
      file={file}
      error={error}
      isLoading={uploading}
      onNameChange={(v) => (name = v)}
      onFileChange={(f) => (file = f)}
      onSubmit={handleUpload}
    />
  </div>
</div>
