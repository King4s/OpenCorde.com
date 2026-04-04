<!--
  @component EmojiManager
  @purpose Manage custom server emojis (upload, list, delete)
  @version 1.0.0
-->
<script lang="ts">
	import { emojiStore } from '$lib/stores/emojis';
	import { onMount } from 'svelte';
	import EmojiGrid from './EmojiGrid.svelte';
	import EmojiUploadForm from './EmojiUploadForm.svelte';

	let { spaceId, onClose }: { spaceId: string; onClose: () => void } = $props();

	let name = $state('');
	let file: File | null = $state(null);
	let uploading = $state(false);
	let error = $state('');

	onMount(() => emojiStore.fetchEmojis(spaceId));

	async function handleUpload() {
		if (!name.trim() || !file) {
			error = 'Please enter a name and select a file';
			return;
		}
		if (name.length < 2 || name.length > 32) {
			error = 'Name must be 2-32 characters';
			return;
		}
		if (!/^[a-z0-9_]+$/.test(name)) {
			error = 'Name can only contain lowercase letters, numbers, and underscores';
			return;
		}
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

<div class="backdrop" onclick={onClose} role="presentation" onkeydown={(e) => e.key === 'Escape' && onClose()}>
	<div class="modal" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.key === 'Escape' && onClose()} role="dialog" aria-modal="true" tabindex="-1">
		<div class="header">
			<h2>Custom Emojis</h2>
			<button onclick={onClose} aria-label="Close modal">✕</button>
		</div>
		<div class="body">
			<EmojiGrid
				emojis={$emojiStore.emojis}
				onDelete={handleDelete}
			/>

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
		padding: 4px;
		border-radius: 4px;
		transition: background-color 0.15s;
	}

	.header button:hover {
		background: #35373c;
	}

	.body {
		overflow-y: auto;
		padding: 16px 20px;
		display: flex;
		flex-direction: column;
		gap: 16px;
	}
</style>
