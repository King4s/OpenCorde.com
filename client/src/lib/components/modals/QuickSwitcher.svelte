<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { channels, currentChannelId } from '$lib/stores/channels';

	interface Props {
		onClose: () => void;
	}

	let { onClose }: Props = $props();
	let query = $state('');
	let inputEl: HTMLInputElement;

	const results = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const list = $channels.filter((c) => c.channel_type === 0);
		if (!q) return list.slice(0, 8);
		return list.filter((c) => c.name.toLowerCase().includes(q)).slice(0, 8);
	});

	onMount(() => {
		inputEl?.focus();
		function handleKeydown(e: KeyboardEvent) {
			if (e.key === 'Escape') onClose();
		}
		document.addEventListener('keydown', handleKeydown);
		return () => document.removeEventListener('keydown', handleKeydown);
	});

	function openChannel(channelId: string) {
		const active = $currentChannelId;
		if (active === channelId) {
			onClose();
			return;
		}
		const sid = window.location.pathname.match(/\/servers\/([^/]+)/)?.[1];
		if (sid) {
			goto(`/servers/${sid}/channels/${channelId}`);
		}
		onClose();
	}
</script>

<div class="fixed inset-0 z-50 flex items-start justify-center pt-24">
	<button
		type="button"
		class="absolute inset-0 bg-black/60"
		aria-label="Close quick switcher"
		onclick={onClose}
	></button>
	<div class="relative z-10 w-full max-w-lg bg-gray-800 border border-gray-700 rounded-xl shadow-2xl p-4">
		<div class="flex items-center justify-between gap-3 mb-3">
			<h2 class="text-sm font-semibold text-white">Quick Switcher</h2>
			<button type="button" class="text-gray-400 hover:text-white text-sm" onclick={onClose}>✕</button>
		</div>

		<input
			bind:this={inputEl}
			bind:value={query}
			type="text"
			placeholder="Search channels"
			class="w-full px-3 py-2 rounded bg-gray-900 border border-gray-700 text-white text-sm focus:outline-none focus:border-indigo-500"
		/>

		<div class="mt-3 max-h-72 overflow-y-auto space-y-1">
			{#if results.length === 0}
				<p class="text-sm text-gray-500 px-2 py-3">No channels found.</p>
			{:else}
				{#each results as channel (channel.id)}
					<button
						type="button"
						onclick={() => openChannel(channel.id)}
						class="w-full text-left px-3 py-2 rounded hover:bg-gray-700 flex items-center justify-between gap-3"
					>
						<span class="text-sm text-white truncate"># {channel.name}</span>
						{#if $currentChannelId === channel.id}
							<span class="text-xs text-indigo-300">current</span>
						{/if}
					</button>
				{/each}
			{/if}
		</div>
	</div>
</div>
