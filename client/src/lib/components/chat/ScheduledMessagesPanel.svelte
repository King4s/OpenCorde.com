<script lang="ts">
	import type { ScheduledMessage } from '$lib/api/types';
	import api from '$lib/api/client';
	import { onMount } from 'svelte';

	interface Props {
		onClose: () => void;
	}

	let { onClose }: Props = $props();
	let messages = $state<ScheduledMessage[]>([]);
	let loading = $state(true);
	let error = $state('');
	let editingId = $state<string | null>(null);
	let editContent = $state('');
	let editTime = $state('');

	async function loadMessages() {
		loading = true;
		error = '';
		try {
			messages = await api.get<ScheduledMessage[]>('/users/@me/scheduled-messages');
		} catch (err: any) {
			error = err.message ?? 'Failed to load scheduled messages';
		} finally {
			loading = false;
		}
	}

	async function cancelMessage(id: string) {
		try {
			await api.delete(`/scheduled-messages/${id}`);
			messages = messages.filter(m => m.id !== id);
		} catch (err: any) {
			error = err.message ?? 'Failed to cancel';
		}
	}

	async function saveEdit(id: string) {
		if (!editContent.trim() && !editTime) return;
		try {
			const body: any = {};
			if (editContent.trim()) body.content = editContent.trim();
			if (editTime) body.scheduled_at = editTime;
			const updated = await api.patch<ScheduledMessage>(`/scheduled-messages/${id}`, body);
			messages = messages.map(m => m.id === id ? updated : m);
			editingId = null;
			editContent = '';
			editTime = '';
		} catch (err: any) {
			error = err.message ?? 'Failed to update';
		}
	}

	function startEdit(msg: ScheduledMessage) {
		editingId = msg.id;
		editContent = msg.content;
		// Convert RFC3339 to datetime-local format
		const d = new Date(msg.scheduled_at);
		editTime = d.toISOString().slice(0, 16);
	}

	function formatTime(rfc3339: string): string {
		const d = new Date(rfc3339);
		return d.toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit',
		});
	}

	function formatChannel(channelId: string): string {
		return '#' + channelId.slice(-4);
	}

	onMount(() => {
		loadMessages();
	});
</script>

<div class="w-72 flex-shrink-0 border-l border-gray-900 bg-gray-750 flex flex-col h-full">
	<div class="h-12 px-3 flex items-center justify-between border-b border-gray-900 flex-shrink-0">
		<span class="text-sm font-semibold text-gray-200">⏰ Scheduled</span>
		<button onclick={onClose} class="text-gray-400 hover:text-white text-sm">✕</button>
	</div>

	<div class="flex-1 overflow-y-auto p-2 space-y-2">
		{#if loading}
			<p class="text-gray-400 text-xs text-center py-4">Loading...</p>
		{:else if error}
			<p class="text-red-400 text-xs text-center py-4">{error}</p>
		{:else if messages.length === 0}
			<p class="text-gray-500 text-xs text-center py-4">No scheduled messages</p>
		{:else}
			{#each messages as msg (msg.id)}
				<div class="bg-gray-700 rounded p-2.5 text-xs space-y-1.5">
					<div class="flex items-center justify-between">
						<span class="text-gray-400">{formatChannel(msg.channel_id)}</span>
						<span class="text-indigo-300">{formatTime(msg.scheduled_at)}</span>
					</div>

					{#if editingId === msg.id}
						<textarea
							bind:value={editContent}
							class="w-full bg-gray-800 text-white text-xs rounded p-1.5 border border-gray-600 focus:border-indigo-400 focus:outline-none resize-none"
							rows="2"
						></textarea>
						<input
							type="datetime-local"
							bind:value={editTime}
							class="w-full bg-gray-800 text-white text-xs rounded p-1 border border-gray-600 focus:border-indigo-400 focus:outline-none"
						/>
						<div class="flex gap-1.5">
							<button
								onclick={() => saveEdit(msg.id)}
								class="flex-1 text-indigo-400 hover:text-indigo-300 text-center py-0.5 rounded hover:bg-gray-600/50"
							>Save</button>
							<button
								onclick={() => { editingId = null; editContent = ''; editTime = ''; }}
								class="flex-1 text-gray-400 hover:text-gray-300 text-center py-0.5 rounded hover:bg-gray-600/50"
							>Cancel</button>
						</div>
					{:else}
						<p class="text-gray-200 break-words">{msg.content}</p>
						<div class="flex gap-1.5">
							<button
								onclick={() => startEdit(msg)}
								class="text-indigo-400 hover:text-indigo-300"
							>Edit</button>
							<button
								onclick={() => cancelMessage(msg.id)}
								class="text-red-400 hover:text-red-300"
							>Cancel</button>
						</div>
					{/if}
				</div>
			{/each}
		{/if}
	</div>

	<div class="p-2 border-t border-gray-900 flex-shrink-0">
		<button
			onclick={loadMessages}
			class="w-full text-gray-400 hover:text-gray-300 text-xs py-1 rounded hover:bg-gray-700/50"
		>
			🔄 Refresh
		</button>
	</div>
</div>
