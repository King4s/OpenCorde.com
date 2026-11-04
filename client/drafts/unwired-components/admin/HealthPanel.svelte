<script lang="ts">
	/**
	 * @file Health Panel
	 * @purpose Instance health dashboard showing real-time status of all core services
	 * @version 1.0.0
	 */
	import api from '$lib/api/client';
	import type { InstanceHealth, ServiceHealth } from '$lib/api/types';

	let health = $state<InstanceHealth | null>(null);
	let loading = $state(false);
	let error = $state('');
	let lastRefresh = $state('');

	async function loadHealth() {
		loading = true;
		error = '';
		try {
			health = await api.get<InstanceHealth>('/admin/health');
			lastRefresh = new Date().toLocaleTimeString();
		} catch (e: any) {
			error = e.message?.includes('403')
				? 'Access denied. Admin privileges required.'
				: (e.message ?? 'Failed to load health data');
			health = null;
		} finally {
			loading = false;
		}
	}

	function colorClass(color: string): string {
		switch (color) {
			case 'green': return 'bg-green-500/20 border-green-500/30 text-green-400';
			case 'amber': return 'bg-amber-500/20 border-amber-500/30 text-amber-400';
			case 'red': return 'bg-red-500/20 border-red-500/30 text-red-400';
			default: return 'bg-gray-700/50 border-gray-600/30 text-gray-400';
		}
	}

	function dotColor(color: string): string {
		switch (color) {
			case 'green': return 'bg-green-400';
			case 'amber': return 'bg-amber-400';
			case 'red': return 'bg-red-400';
			default: return 'bg-gray-500';
		}
	}

	function statusLabel(svc: ServiceHealth): string {
		if (svc.status === 'ok') return 'Healthy';
		if (svc.status === 'degraded') return 'Degraded';
		if (svc.status === 'error') return 'Down';
		if (svc.status === 'disabled') return 'Disabled';
		return svc.status;
	}

	function formatLatency(ms: number | null): string {
		if (ms === null) return '—';
		if (ms < 1) return '<1 ms';
		if (ms < 1000) return `${ms} ms`;
		return `${(ms / 1000).toFixed(1)} s`;
	}

	function serviceName(key: string): string {
		const names: Record<string, string> = {
			api: 'API Server',
			database: 'Database',
			redis: 'Redis',
			storage: 'Object Storage',
			livekit: 'LiveKit',
			smtp: 'SMTP Email',
			bridge: 'Discord Bridge',
			background_jobs: 'Background Jobs',
		};
		return names[key] ?? key;
	}

	function serviceIcon(key: string): string {
		const icons: Record<string, string> = {
			api: '⬡',
			database: '⬢',
			redis: '◈',
			storage: '⊞',
			livekit: '◉',
			smtp: '✉',
			bridge: '⤤',
			background_jobs: '⚙',
		};
		return icons[key] ?? '•';
	}

	$effect(() => {
		loadHealth();
	});

	const serviceKeys = ['api', 'database', 'redis', 'storage', 'livekit', 'smtp', 'bridge', 'background_jobs'] as const;

	function countByColor(color: string): number {
		if (!health) return 0;
		const h = health;
		return serviceKeys.filter(k => h[k].color === color).length;
	}

	let healthyCount = $derived(countByColor('green'));
	let degradedCount = $derived(countByColor('amber'));
	let downCount = $derived(countByColor('red'));
</script>

<div class="space-y-4">
	<!-- Header -->
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<h2 class="text-white font-medium text-lg">Instance Health</h2>
			<p class="text-gray-500 text-sm mt-0.5">
				{#if health}
					Checked at {new Date(health.checked_at).toLocaleString()}
				{:else if loading}
					Checking services...
				{:else}
					Status unknown
				{/if}
			</p>
		</div>
		<button
			onclick={loadHealth}
			disabled={loading}
			class="px-3 py-1.5 text-xs rounded bg-gray-700 text-gray-200 hover:bg-gray-600 disabled:opacity-50 transition-colors"
		>
			{loading ? 'Checking…' : '↻ Refresh'}
		</button>
	</div>

	{#if error}
		<div class="px-4 py-3 bg-gray-900/40 border border-gray-700/50 rounded text-gray-300 text-sm">
			{error}
		</div>
	{/if}

	<!-- Summary bar -->
	{#if health}
		<div class="flex flex-wrap gap-3 text-sm">
			<span class="flex items-center gap-1.5">
				<span class="w-2 h-2 rounded-full bg-green-400 inline-block"></span>
				<span class="text-green-400">{healthyCount} healthy</span>
			</span>
			{#if degradedCount > 0}
				<span class="flex items-center gap-1.5">
					<span class="w-2 h-2 rounded-full bg-amber-400 inline-block"></span>
					<span class="text-amber-400">{degradedCount} degraded</span>
				</span>
			{/if}
			{#if downCount > 0}
				<span class="flex items-center gap-1.5">
					<span class="w-2 h-2 rounded-full bg-red-400 inline-block"></span>
					<span class="text-red-400">{downCount} down</span>
				</span>
			{/if}
		</div>
	{/if}

	<!-- Service cards -->
	{#if health}
		<div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
			{#each serviceKeys as key}
				{@const svc = health[key]}
				<div class="rounded-lg border p-4 {colorClass(svc.color)}">
					<div class="flex items-start justify-between gap-2">
						<div class="flex items-center gap-2">
							<span class="text-lg">{serviceIcon(key)}</span>
							<span class="font-medium text-sm">{serviceName(key)}</span>
						</div>
						<div class="flex items-center gap-1.5">
							<span class="w-2 h-2 rounded-full {dotColor(svc.color)} inline-block"></span>
							<span class="text-xs font-medium">{statusLabel(svc)}</span>
						</div>
					</div>
					<div class="mt-2 text-xs opacity-80">{svc.detail}</div>
					<div class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs opacity-60">
						<span>Latency: {formatLatency(svc.latency_ms)}</span>
						{#if svc.error}
							<span class="text-red-300/70 break-all">{svc.error}</span>
						{/if}
					</div>
				</div>
			{/each}
		</div>
	{/if}

	<!-- Empty/loading state -->
	{#if !health && !error && !loading}
		<div class="py-12 text-center text-gray-500 text-sm">
			No health data available.
			<button onclick={loadHealth} class="text-gray-400 hover:text-white underline ml-1">Check now</button>
		</div>
	{/if}
</div>
