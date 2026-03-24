<script lang="ts">
	/**
	 * @file Pins panel component
	 * @purpose Show pinned messages for a channel, allow unpin
	 */
	import api from '$lib/api/client';
	import { renderMarkdown } from '$lib/utils/markdown';

	interface PinnedMessage {
		message_id: string;
		author_username: string;
		content: string;
		created_at: string;
		pinned_at: string;
	}

	interface Props {
		channelId: string;
		onClose: () => void;
	}

	let { channelId, onClose }: Props = $props();
	let pins = $state<PinnedMessage[]>([]);
	let loading = $state(true);
	let error = $state('');

	$effect(() => {
		api
			.get<PinnedMessage[]>(`/channels/${channelId}/pins`)
			.then((list) => {
				pins = list;
				loading = false;
			})
			.catch((e) => {
				error = e.message ?? 'Failed to load pins';
				loading = false;
			});
	});

	async function handleUnpin(messageId: string) {
		try {
			await api.delete(`/channels/${channelId}/pins/${messageId}`);
			pins = pins.filter((p) => p.message_id !== messageId);
		} catch (e: any) {
			error = e.message ?? 'Failed to unpin';
		}
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleDateString();
	}
</script>

<!-- Slide-in panel from the right -->
<div class="absolute top-0 right-0 h-full w-72 bg-gray-800 border-l border-gray-700 z-20 flex flex-col shadow-xl">
	<div class="flex items-center justify-between px-4 py-3 border-b border-gray-700 flex-shrink-0">
		<h3 class="font-semibold text-white text-sm flex items-center gap-2">
			📌 Pinned Messages
		</h3>
		<button onclick={onClose} class="text-gray-400 hover:text-white">✕</button>
	</div>
	<div class="flex-1 overflow-y-auto p-3 space-y-2">
		{#if loading}
			<p class="text-gray-500 text-sm">Loading…</p>
		{:else if error}
			<p class="text-red-400 text-sm">{error}</p>
		{:else if pins.length === 0}
			<p class="text-gray-500 text-sm">No pinned messages in this channel.</p>
		{:else}
			{#each pins as pin (pin.message_id)}
				<div class="bg-gray-700/50 rounded p-2 group">
					<div class="flex items-baseline justify-between mb-1">
						<span class="text-xs font-semibold text-indigo-400">{pin.author_username}</span>
						<span class="text-xs text-gray-500">{formatDate(pin.created_at)}</span>
					</div>
					<div class="text-gray-300 text-sm prose-sm break-words">
						{@html renderMarkdown(pin.content)}
					</div>
					<button
						onclick={() => handleUnpin(pin.message_id)}
						class="mt-1.5 text-xs text-gray-500 hover:text-red-400 opacity-0 group-hover:opacity-100 transition-opacity"
					>
						Unpin
					</button>
				</div>
			{/each}
		{/if}
	</div>
</div>
