<script lang="ts">
	/**
	 * @file Search modal component
	 * @purpose Full-text message search, channel lookup, member search
	 * @version 2.0.0 — channels + members results
	 */
	import { goto } from '$app/navigation';
	import { get } from 'svelte/store';
	import api from '$lib/api/client';
	import { channels } from '$lib/stores/channels';
	import { members } from '$lib/stores/members';

	interface SearchResult {
		message_id: string;
		channel_id: string;
		server_id: string;
		author_id: string;
		content: string;
		score: number;
	}

	interface SearchResponse {
		results: SearchResult[];
		count: number;
	}

	interface Props {
		channelId: string;
		spaceId?: string;
		onClose: () => void;
	}

	let { channelId, spaceId, onClose }: Props = $props();
	let query = $state('');
	let results = $state<SearchResult[]>([]);
	let searching = $state(false);
	let searched = $state(false);
	let error = $state('');
	let inputEl: HTMLInputElement;

	const channelResults = $derived.by(() => {
		const q = query.toLowerCase().trim();
		if (!q) return [];
		const ch = get(channels).filter(c => c.name.toLowerCase().includes(q));
		return ch.slice(0, 5);
	});

	const memberResults = $derived.by(() => {
		const q = query.toLowerCase().trim();
		if (!q) return [];
		const mem = get(members).filter(m => m.username.toLowerCase().includes(q));
		return mem.slice(0, 5);
	});

	$effect(() => {
		inputEl?.focus();
	});

	async function handleSearch(e?: Event) {
		e?.preventDefault();
		if (!query.trim()) return;
		searching = true;
		error = '';
		try {
			const params = new URLSearchParams({ q: query.trim(), limit: '25' });
			if (channelId) params.set('channel_id', channelId);
			const res = await api.get<SearchResponse>(`/search?${params}`);
			results = res.results;
			searched = true;
		} catch (e: any) {
			error = e.message ?? 'Search failed';
		} finally {
			searching = false;
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onClose();
	}
</script>

<!-- Backdrop -->
<div
	class="fixed inset-0 z-50 bg-black/60 flex items-start justify-center pt-20"
	role="button"
	tabindex="-1"
	aria-label="Close search"
	onclick={onClose}
	onkeydown={handleKeydown}
>
	<!-- Modal -->
	<div
		class="bg-gray-800 border border-gray-700 rounded-xl shadow-2xl w-full max-w-2xl mx-4"
		role="dialog"
		tabindex="0"
		aria-label="Search messages"
		onclick={(e) => e.stopPropagation()}
		onkeydown={(e) => e.stopPropagation()}
	>
		<form onsubmit={handleSearch} class="p-4 border-b border-gray-700">
			<div class="flex gap-2">
				<input
					bind:this={inputEl}
					type="text"
					bind:value={query}
					placeholder="Search messages…"
					class="flex-1 px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm placeholder-gray-500 focus:outline-none focus:border-gray-500"
				/>
				<button
					type="submit"
					disabled={searching || !query.trim()}
					class="px-4 py-2 bg-gray-600 hover:bg-gray-700 disabled:opacity-50 text-white text-sm rounded transition-colors"
				>
					{searching ? '…' : 'Search'}
				</button>
			</div>
		</form>

		<div class="max-h-96 overflow-y-auto p-2">
			{#if error}
				<p class="text-gray-400 text-sm p-3">{error}</p>
			{:else if searching}
				<p class="text-gray-500 text-sm p-3">Searching…</p>
			{:else}
				<!-- Channel results -->
				{#if channelResults.length > 0}
					<div class="border-b border-gray-700 pb-2 mb-2">
						<p class="text-gray-500 text-xs px-3 py-1 font-semibold">Channels</p>
						{#each channelResults as ch (ch.id)}
							<button
								class="w-full text-left px-3 py-2 rounded hover:bg-gray-700/50 transition-colors"
								onclick={() => {
									goto(`/servers/${spaceId || ch.id}/channels/${ch.id}`);
									onClose();
								}}
							>
								<p class="text-gray-300 text-sm">#{ch.name}</p>
							</button>
						{/each}
					</div>
				{/if}

				<!-- Member results -->
				{#if memberResults.length > 0}
					<div class="border-b border-gray-700 pb-2 mb-2">
						<p class="text-gray-500 text-xs px-3 py-1 font-semibold">Members</p>
						{#each memberResults as mem (mem.user_id)}
							<div class="px-3 py-2 rounded hover:bg-gray-700/50 transition-colors">
								<p class="text-gray-300 text-sm">@{mem.username}</p>
							</div>
						{/each}
					</div>
				{/if}

				<!-- Message results -->
				{#if searched && results.length === 0 && channelResults.length === 0 && memberResults.length === 0}
					<p class="text-gray-500 text-sm p-3">No results for "{query}"</p>
				{:else if results.length > 0}
					<div>
						<p class="text-gray-500 text-xs px-3 py-1 font-semibold">Messages</p>
						{#each results as r (r.message_id)}
							<div class="px-3 py-2 rounded hover:bg-gray-700/50 transition-colors">
								<p class="text-gray-300 text-sm leading-relaxed">{r.content}</p>
								<p class="text-gray-600 text-xs mt-0.5">message #{r.message_id}</p>
							</div>
						{/each}
					</div>
				{:else if !searched && query.length === 0}
					<p class="text-gray-600 text-sm p-3">Type to search messages, channels, or members.</p>
				{/if}
			{/if}
		</div>
	</div>
</div>
