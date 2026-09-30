<script lang="ts">
	/**
	 * @file Browse Channels page — dedicated surface for exploring server channels
	 * @purpose Shows all channels with topics, search, and type filters for large servers
	 */
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { channels, fetchChannels, currentChannelId } from '$lib/stores/channels';
	import { currentSpace } from '$lib/stores/servers';
	import type { Channel } from '$lib/api/types';

	let spaceId = $state('');
	let query = $state('');
	let activeFilter = $state<number | null>(null);
	let inputEl: HTMLInputElement;

	$effect(() => {
		const sid = $page.params.serverId;
		if (sid && sid !== spaceId) {
			spaceId = sid;
			fetchChannels(sid).catch(() => {});
		}
	});

	const channelTypeLabels: Record<number, string> = {
		0: 'Text',
		1: 'Voice',
		2: 'Category',
		3: 'Stage',
		4: 'Announcement',
		5: 'Forum',
	};

	const channelTypeIcons: Record<number, string> = {
		0: '#',
		1: '🔊',
		2: '📁',
		3: '📢',
		4: '📤',
		5: '📋',
	};

	const filteredChannels = $derived.by(() => {
		let list = $channels;
		if (activeFilter !== null) {
			list = list.filter((c) => c.channel_type === activeFilter);
		}
		const q = query.trim().toLowerCase();
		if (q) {
			list = list.filter(
				(c) =>
					c.name.toLowerCase().includes(q) ||
					(c.topic ?? '').toLowerCase().includes(q)
			);
		}
		return list.sort((a, b) => a.position - b.position);
	});

	const categories = $derived.by(() => {
		const cats = filteredChannels.filter((c) => c.channel_type === 2);
		return cats.length > 0 ? cats : [{ id: '__uncategorized', name: 'Channels', channel_type: 2, position: -1 } as Channel];
	});

	function channelsInCategory(catId: string | null) {
		if (catId === '__uncategorized') {
			return filteredChannels.filter((c) => c.channel_type !== 2 && c.parent_id === null);
		}
		return filteredChannels.filter((c) => c.parent_id === catId && c.channel_type !== 2);
	}

	function openChannel(channel: Channel) {
		if (channel.channel_type === 5) {
			goto(`/servers/${spaceId}/forum/${channel.id}`);
		} else if (channel.channel_type === 0 || channel.channel_type === 4) {
			goto(`/servers/${spaceId}/channels/${channel.id}`);
		} else if (channel.channel_type === 1 || channel.channel_type === 3) {
			// Voice/stage channels — navigate to channel page or join voice
			goto(`/servers/${spaceId}/channels/${channel.id}`);
		}
	}

	function goBack() {
		const cur = $currentChannelId;
		if (cur) {
			goto(`/servers/${spaceId}/channels/${cur}`);
		} else {
			goto(`/servers/${spaceId}`);
		}
	}

	onMount(() => {
		inputEl?.focus();
		function handleKeydown(e: KeyboardEvent) {
			if (e.key === 'Escape') goBack();
		}
		document.addEventListener('keydown', handleKeydown);
		return () => document.removeEventListener('keydown', handleKeydown);
	});
</script>

<div class="flex h-full flex-col bg-gray-900">
	<!-- Header -->
	<div class="flex items-center gap-3 border-b border-gray-800 px-4 py-3">
		<button
			type="button"
			onclick={goBack}
			class="rounded p-1 text-gray-400 hover:bg-gray-800 hover:text-white"
			aria-label="Back"
		>
			<svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
			</svg>
		</button>
		<div class="min-w-0 flex-1">
			<h1 class="truncate text-sm font-semibold text-white">
				Browse Channels — {$currentSpace?.name ?? 'Space'}
			</h1>
			<p class="truncate text-xs text-gray-500">
				{$channels.length} channel{$channels.length === 1 ? '' : 's'}
			</p>
		</div>
	</div>

	<!-- Search & Filters -->
	<div class="border-b border-gray-800 px-4 py-3 space-y-3">
		<div class="relative">
			<svg
				class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-500"
				fill="none"
				stroke="currentColor"
				viewBox="0 0 24 24"
			>
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
				/>
			</svg>
			<input
				bind:this={inputEl}
				bind:value={query}
				type="text"
				placeholder="Search by name or topic"
				class="w-full rounded-lg bg-gray-800 py-2 pl-9 pr-3 text-sm text-white placeholder-gray-500 outline-none ring-1 ring-gray-700 focus:ring-indigo-500"
			/>
			{#if query}
				<button
					type="button"
					onclick={() => (query = '')}
					class="absolute right-2 top-1/2 -translate-y-1/2 rounded p-0.5 text-gray-500 hover:text-white"
				>
					✕
				</button>
			{/if}
		</div>

		<div class="flex flex-wrap gap-1.5">
			<button
				type="button"
				onclick={() => (activeFilter = activeFilter === null ? null : null)}
				class="rounded-full px-3 py-1 text-xs font-medium transition-colors {activeFilter === null ? 'bg-indigo-600 text-white' : 'bg-gray-800 text-gray-400 hover:bg-gray-700 hover:text-gray-200'}"
			>
				All
			</button>
			{#each [0, 1, 3, 5] as type (type)}
				<button
					type="button"
					onclick={() => (activeFilter = activeFilter === type ? null : type)}
					class="rounded-full px-3 py-1 text-xs font-medium transition-colors {activeFilter === type ? 'bg-indigo-600 text-white' : 'bg-gray-800 text-gray-400 hover:bg-gray-700 hover:text-gray-200'}"
				>
					{channelTypeIcons[type]} {channelTypeLabels[type]}
				</button>
			{/each}
		</div>
	</div>

	<!-- Channel Grid -->
	<div class="min-h-0 flex-1 overflow-y-auto px-4 py-3">
		{#if filteredChannels.length === 0}
			<div class="flex flex-col items-center justify-center py-12 text-center">
				<p class="text-sm text-gray-500">No channels match your search.</p>
			</div>
		{:else}
			{#each categories as category (category.id)}
				{@const chs = channelsInCategory(category.id === '__uncategorized' ? '__uncategorized' : category.id)}
				{#if chs.length > 0}
					<div class="mb-5">
						{#if category.id !== '__uncategorized'}
							<h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-gray-500">
								{channelTypeIcons[2]} {category.name}
							</h3>
						{/if}
						<div class="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
							{#each chs as channel (channel.id)}
								<button
									type="button"
									onclick={() => openChannel(channel)}
									class="group flex flex-col items-start rounded-lg bg-gray-800 p-3 text-left ring-1 ring-gray-700 transition-colors hover:bg-gray-750 hover:ring-gray-600"
								>
									<div class="flex w-full items-center gap-2">
										<span class="text-sm text-gray-400">
											{channelTypeIcons[channel.channel_type] ?? '#'}
										</span>
										<span class="min-w-0 flex-1 truncate text-sm font-medium text-white">
											{channel.name}
										</span>
										{#if channel.nsfw}
											<span class="rounded bg-red-900/40 px-1.5 py-0.5 text-[10px] font-semibold text-red-400">NSFW</span>
										{/if}
									</div>
									{#if channel.topic}
										<p class="mt-1 line-clamp-2 text-xs text-gray-500">
											{channel.topic}
										</p>
									{/if}
									<div class="mt-2 flex items-center gap-2 text-[10px] text-gray-600">
										<span>{channelTypeLabels[channel.channel_type] ?? 'Channel'}</span>
										{#if channel.slowmode_delay > 0}
											<span>• Slowmode {channel.slowmode_delay}s</span>
										{/if}
										{#if channel.e2ee_enabled}
											<span>• E2EE</span>
										{/if}
									</div>
								</button>
							{/each}
						</div>
					</div>
				{/if}
			{/each}
		{/if}
	</div>
</div>

<style>
	:global(.bg-gray-750) {
		background-color: #2f3136;
	}
	.line-clamp-2 {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
</style>
