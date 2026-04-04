<!--
  @file RecordingsPanel.svelte
  @purpose Show past recordings for a channel with download links
  @version 1.0.0
-->
<script lang="ts">
	import api from '$lib/api/client';

	interface Recording {
		id: string;
		started_by: string;
		status: string;
		file_path: string | null;
		started_at: string;
		stopped_at: string | null;
	}

	let { channelId, onClose }: { channelId: string; onClose: () => void } = $props();

	let recordings = $state<Recording[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	$effect(() => {
		if (!channelId) return;
		loading = true;
		error = null;
		api.get<Recording[]>(`/channels/${channelId}/recordings`)
			.then((data) => { recordings = data; })
			.catch(() => { error = 'Failed to load recordings.'; })
			.finally(() => { loading = false; });
	});

	function formatDuration(start: string, stop: string | null): string {
		if (!stop) return 'In progress';
		const ms = new Date(stop).getTime() - new Date(start).getTime();
		const s = Math.floor(ms / 1000);
		const m = Math.floor(s / 60);
		const h = Math.floor(m / 60);
		if (h > 0) return `${h}h ${m % 60}m`;
		if (m > 0) return `${m}m ${s % 60}s`;
		return `${s}s`;
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleString(undefined, {
			month: 'short', day: 'numeric',
			hour: '2-digit', minute: '2-digit'
		});
	}

	function downloadUrl(filePath: string): string {
		// file_path is MinIO object path; route through our API proxy
		return `/api/v1/files/${encodeURIComponent(filePath)}`;
	}
</script>

<div class="w-72 bg-gray-800 border-l border-gray-700 flex flex-col h-full">
	<!-- Header -->
	<div class="flex items-center justify-between px-4 py-3 border-b border-gray-700">
		<h3 class="text-sm font-semibold text-white">Recordings</h3>
		<button
			onclick={onClose}
			class="text-gray-400 hover:text-white transition-colors"
			aria-label="Close recordings panel"
		>✕</button>
	</div>

	<!-- Content -->
	<div class="flex-1 overflow-y-auto p-3 space-y-2">
		{#if loading}
			<p class="text-gray-400 text-xs text-center py-6">Loading…</p>
		{:else if error}
			<p class="text-gray-400 text-xs text-center py-6">{error}</p>
		{:else if recordings.length === 0}
			<p class="text-gray-400 text-xs text-center py-6">No recordings yet.</p>
		{:else}
			{#each recordings as rec (rec.id)}
				<div class="bg-gray-750 rounded p-3 border border-gray-700 space-y-1">
					<div class="flex items-center justify-between">
						<span class="text-xs font-medium {rec.status === 'active' ? 'text-gray-400' : 'text-gray-200'}">
							{rec.status === 'active' ? '⏺ Recording…' : '🎥 Recording'}
						</span>
						<span class="text-xs text-gray-500">{formatDuration(rec.started_at, rec.stopped_at)}</span>
					</div>
					<p class="text-xs text-gray-400">{formatDate(rec.started_at)}</p>
					{#if rec.file_path && rec.status === 'completed'}
						<a
							href={downloadUrl(rec.file_path)}
							download
							class="inline-block mt-1 text-xs text-gray-400 hover:text-gray-300 transition-colors"
							aria-label="Download recording"
						>
							⬇ Download
						</a>
					{/if}
				</div>
			{/each}
		{/if}
	</div>
</div>
