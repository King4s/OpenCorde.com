<!--
  @component SlashCommandManager
  @purpose Register and manage slash commands for a server
  @version 1.0.0
-->
<script lang="ts">
	import { slashCommandsStore } from '$lib/stores/slashCommands.svelte';
	import { onMount } from 'svelte';
	import SlashCommandForm from './SlashCommandForm.svelte';
	import SlashCommandList from './SlashCommandList.svelte';

	let { serverId, onClose }: { serverId: string; onClose: () => void } = $props();

	let newName = $state('');
	let newDescription = $state('');
	let newHandlerUrl = $state('');
	let creating = $state(false);
	let error = $state('');

	onMount(() => slashCommandsStore.fetchCommands(serverId));

	async function handleCreate() {
		if (!newName.trim()) {
			error = 'Command name is required';
			return;
		}
		if (!newHandlerUrl.trim()) {
			error = 'Handler URL is required';
			return;
		}
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

<div class="backdrop" onclick={onClose} role="presentation" onkeydown={(e) => e.key === 'Escape' && onClose()}>
	<div
		class="modal"
		onclick={(e) => e.stopPropagation()}
		onkeydown={(e) => e.key === 'Escape' && onClose()}
		role="dialog"
		aria-modal="true"
		tabindex="-1"
	>
		<div class="header">
			<h2>Slash Commands</h2>
			<button onclick={onClose} aria-label="Close modal">✕</button>
		</div>
		<div class="body">
			<SlashCommandList
				commands={slashCommandsStore.commands}
				onDelete={handleDelete}
			/>

			<SlashCommandForm
				name={newName}
				description={newDescription}
				handlerUrl={newHandlerUrl}
				error={error}
				isLoading={creating}
				onNameChange={(v) => (newName = v)}
				onDescriptionChange={(v) => (newDescription = v)}
				onHandlerUrlChange={(v) => (newHandlerUrl = v)}
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
