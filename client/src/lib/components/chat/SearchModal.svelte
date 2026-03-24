<script lang="ts">
	/**
	 * @file Search modal component
	 * @purpose Full-text message search via Tantivy backend
	 */
	import api from '$lib/api/client';

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
		serverId?: string;
		onClose: () => void;
	}

	let { channelId, serverId, onClose }: Props = $props();
	let query = $state('');
	let results = $state<SearchResult[]>([]);
	let searching = $state(false);
	let searched = $state(false);
	let error = $state('');
	let inputEl: HTMLInputElement;

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
					class="flex-1 px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm placeholder-gray-500 focus:outline-none focus:border-indigo-500"
				/>
				<button
					type="submit"
					disabled={searching || !query.trim()}
					class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white text-sm rounded transition-colors"
				>
					{searching ? '…' : 'Search'}
				</button>
			</div>
		</form>

		<div class="max-h-96 overflow-y-auto p-2">
			{#if error}
				<p class="text-red-400 text-sm p-3">{error}</p>
			{:else if searching}
				<p class="text-gray-500 text-sm p-3">Searching…</p>
			{:else if searched && results.length === 0}
				<p class="text-gray-500 text-sm p-3">No results for "{query}"</p>
			{:else if results.length > 0}
				<p class="text-gray-500 text-xs px-3 py-1">{results.length} result{results.length !== 1 ? 's' : ''}</p>
				{#each results as r (r.message_id)}
					<div class="px-3 py-2 rounded hover:bg-gray-700/50 transition-colors">
						<p class="text-gray-300 text-sm leading-relaxed">{r.content}</p>
						<p class="text-gray-600 text-xs mt-0.5">message #{r.message_id}</p>
					</div>
				{/each}
			{:else}
				<p class="text-gray-600 text-sm p-3">Type to search messages in this channel.</p>
			{/if}
		</div>
	</div>
</div>
