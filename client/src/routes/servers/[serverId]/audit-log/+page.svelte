<script lang="ts">
	/**
	 * @file Audit Log Page
	 * @purpose Display server moderation history with pagination
	 */
	import { browser } from '$app/environment';
	import api from '$lib/api/client';

	interface AuditEntry {
		id: string;
		actor_id: string | null;
		actor_username: string | null;
		action: string;
		target_id: string | null;
		target_type: string | null;
		changes: Record<string, any> | null;
		created_at: string;
	}

	let spaceId = $state('');
	let entries = $state<AuditEntry[]>([]);
	let loading = $state(false);
	let error = $state('');

	if (browser) {
		const match = window.location.pathname.match(/\/servers\/([^/]+)/);
		spaceId = match?.[1] ?? '';
	}

	async function loadEntries() {
		if (!spaceId) return;
		loading = true;
		error = '';
		try {
			entries = await api.get<AuditEntry[]>(`/servers/${spaceId}/audit-log`);
		} catch (e: any) {
			error = e.message ?? 'Failed to load audit log';
			entries = [];
		} finally {
			loading = false;
		}
	}

	async function loadMore() {
		if (!spaceId || entries.length === 0) return;
		loading = true;
		try {
			const lastId = entries[entries.length - 1].id;
			const moreEntries = await api.get<AuditEntry[]>(
				`/servers/${spaceId}/audit-log?before=${lastId}&limit=50`
			);
			entries = [...entries, ...moreEntries];
		} catch (e: any) {
			error = e.message ?? 'Failed to load more entries';
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		if (spaceId) loadEntries();
	});

	function getActionColor(action: string): string {
		if (action.includes('ban')) return 'bg-gray-900/40 text-gray-300';
		if (action.includes('kick')) return 'bg-gray-900/40 text-gray-300';
		if (action.includes('timeout')) return 'bg-gray-900/40 text-gray-300';
		if (action.includes('create')) return 'bg-gray-900/40 text-gray-300';
		if (action.includes('delete')) return 'bg-gray-900/40 text-gray-300';
		if (action.includes('update')) return 'bg-gray-900/40 text-gray-300';
		return 'bg-gray-700 text-gray-300';
	}

	function formatDate(dateStr: string): string {
		const date = new Date(dateStr);
		return date.toLocaleString();
	}

	function getActionLabel(action: string): string {
		return action
			.split('.')
			.map((word) => word.charAt(0).toUpperCase() + word.slice(1))
			.join(' ');
	}
</script>

<div class="flex-1 overflow-y-auto bg-gray-750 p-8">
	<div class="max-w-4xl">
		<div class="flex items-center justify-between mb-6">
			<h1 class="text-xl font-semibold text-white">Audit Log</h1>
			<a
				href="/servers/{spaceId}/settings"
				class="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white text-sm rounded transition-colors"
			>
				Back to Settings
			</a>
		</div>

		{#if error}
			<div class="mb-4 px-3 py-2 bg-gray-900/40 border border-gray-700/50 rounded text-gray-300 text-sm">
				{error}
			</div>
		{/if}

		{#if loading && entries.length === 0}
			<div class="text-gray-400 text-sm">Loading audit log...</div>
		{:else if entries.length === 0}
			<div class="text-gray-400 text-sm">No audit log entries yet.</div>
		{:else}
			<div class="space-y-2">
				{#each entries as entry (entry.id)}
					<div class="px-4 py-3 bg-gray-900 border border-gray-700 rounded">
						<div class="flex items-start justify-between gap-4 mb-2">
							<div class="flex-1">
								<div class="flex items-center gap-2 mb-1">
									<span class="text-xs text-gray-400 font-mono">{entry.actor_username ?? 'Unknown'}</span>
									<span class={`px-2 py-1 rounded text-xs font-medium ${getActionColor(entry.action)}`}>
										{getActionLabel(entry.action)}
									</span>
								</div>
								{#if entry.target_id && entry.target_type}
									<div class="text-xs text-gray-500">
										{entry.target_type}: <span class="font-mono">{entry.target_id}</span>
									</div>
								{/if}
							</div>
							<time class="text-xs text-gray-500 whitespace-nowrap">
								{formatDate(entry.created_at)}
							</time>
						</div>
						{#if entry.changes}
							<div class="text-xs bg-gray-800 rounded p-2 font-mono text-gray-400 overflow-x-auto">
								<pre>{JSON.stringify(entry.changes, null, 2)}</pre>
							</div>
						{/if}
					</div>
				{/each}
			</div>

			{#if !loading}
				<button
					onclick={loadMore}
					class="mt-6 px-4 py-2 bg-gray-600 hover:bg-gray-700 text-white text-sm font-medium rounded transition-colors"
				>
					Load More
				</button>
			{:else}
				<div class="mt-6 text-gray-400 text-sm">Loading...</div>
			{/if}
		{/if}
	</div>
</div>
