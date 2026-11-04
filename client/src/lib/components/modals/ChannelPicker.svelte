<script lang="ts">
	/**
	 * @file ChannelPicker modal for selecting a destination when forwarding messages
	 * @purpose Shows servers and their text channels for the forward action
	 */
	import { onMount } from 'svelte';
	import api from '$lib/api/client';
	import type { Space, Channel } from '$lib/api/types';
	import { fly } from 'svelte/transition';

	interface Props {
		onSelect: (channelId: string, channelName: string) => void;
		onClose: () => void;
	}

	let { onSelect, onClose }: Props = $props();

	let servers = $state<{ server: Space; channels: Channel[] }[]>([]);
	let loading = $state(true);
	let error = $state('');
	let searchQuery = $state('');

	const filteredServers = $derived.by(() => {
		if (!searchQuery.trim()) return servers;
		const q = searchQuery.toLowerCase();
		return servers
			.map((s) => ({
				server: s.server,
				channels: s.channels.filter(
					(c) =>
						c.name.toLowerCase().includes(q) ||
						s.server.name.toLowerCase().includes(q),
				),
			}))
			.filter((s) => s.channels.length > 0 || s.server.name.toLowerCase().includes(q));
	});

	onMount(async () => {
		try {
			const serverList = await api.get<Space[]>('/servers');
			const results = await Promise.all(
				serverList.map(async (srv) => {
					try {
						const ch = await api.get<Channel[]>(`/servers/${srv.id}/channels`);
						return { server: srv, channels: ch.filter((c) => c.channel_type === 0) };
					} catch {
						return { server: srv, channels: [] as Channel[] };
					}
				}),
			);
			servers = results.filter((r) => r.channels.length > 0);
		} catch (e: any) {
			error = e.message || 'Failed to load servers';
		} finally {
			loading = false;
		}
	});

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onClose();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- Backdrop -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="fixed inset-0 bg-black/60 z-50 flex items-start justify-center pt-[15vh]"
	onclick={onClose}
	onkeydown={(e) => { if (e.key === 'Escape') onClose(); }}
>
	<!-- Modal (stop propagation so clicks inside don't close) -->
	<div
		class="bg-gray-900 border border-gray-700 rounded-lg w-[440px] max-h-[60vh] flex flex-col shadow-2xl"
		onclick={(e) => e.stopPropagation()}
		onkeydown={(e) => e.stopPropagation()}
		transition:fly={{ y: -20, duration: 150 }}
	>
		<!-- Header -->
		<div class="px-4 py-3 border-b border-gray-700/60 flex items-center justify-between shrink-0">
			<h2 class="text-base font-semibold text-gray-200">Forward to Channel</h2>
			<button
				class="text-gray-500 hover:text-gray-300 text-lg leading-none px-1"
				onclick={onClose}
				aria-label="Close"
			>✕</button>
		</div>

		<!-- Search -->
		<div class="px-4 py-2 border-b border-gray-700/40 shrink-0">
			<input
				type="text"
				class="w-full bg-gray-800 border border-gray-600 rounded px-3 py-1.5 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-blue-500"
				placeholder="Search channels..."
				bind:value={searchQuery}
			/>
		</div>

		<!-- List -->
		<div class="overflow-y-auto flex-1 px-2 py-1">
			{#if loading}
				<div class="text-center text-gray-500 py-8 text-sm">Loading servers...</div>
			{:else if error}
				<div class="text-center text-red-400 py-8 text-sm">{error}</div>
			{:else if filteredServers.length === 0}
				<div class="text-center text-gray-500 py-8 text-sm">No channels found</div>
			{:else}
				{#each filteredServers as { server, channels }}
					<div class="mb-1">
						<div class="text-xs font-semibold text-gray-500 uppercase tracking-wider px-2 py-1.5">
							{server.name}
						</div>
						{#each channels as ch}
							<button
								class="w-full text-left px-2 py-1.5 rounded text-sm text-gray-300 hover:bg-gray-800 hover:text-gray-100 flex items-center gap-2 transition-colors"
								onclick={() => onSelect(ch.id, `#${ch.name} in ${server.name}`)}
							>
								<span class="text-gray-500 text-xs w-4">#</span>
								<span>{ch.name}</span>
							</button>
						{/each}
					</div>
				{/each}
			{/if}
		</div>
	</div>
</div>
