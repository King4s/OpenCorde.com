<script lang="ts">
	/**
	 * @file Admin Dashboard
	 * @purpose Manage instance resources: users, instances, statistics, rate limiting
	 * @version 2.0.0 — adds storage stats and rate limit config
	 */
	import api from '$lib/api/client';
	import type { InstanceStats, AdminUserRow, AdminServerRow, RateLimitConfig } from '$lib/api/types';
	import UsersTable from './UsersTable.svelte';
	import ServersTable from './ServersTable.svelte';
	import DeleteConfirmModal from './DeleteConfirmModal.svelte';

	let stats = $state<InstanceStats | null>(null);
	let users = $state<AdminUserRow[]>([]);
	let servers = $state<AdminServerRow[]>([]);
	let rateLimits = $state<RateLimitConfig | null>(null);
	let activeTab = $state<'users' | 'instances' | 'rate-limits'>('users');
	let loading = $state(false);
	let error = $state('');
	let userLimit = $state(50);
	let userOffset = $state(0);
	let serverLimit = $state(50);
	let serverOffset = $state(0);
	let deleteConfirm = $state<{ type: 'user' | 'server'; id: string; name: string } | null>(null);
	let rlRps = $state(100);
	let rlBurst = $state(200);
	let rlEnabled = $state(true);
	let rlSaving = $state(false);
	let rlError = $state('');

	$effect(() => {
		loadStats();
		loadUsers();
	});

	async function loadStats() {
		try {
			stats = await api.get<InstanceStats>('/admin/stats');
			error = '';
		} catch (e: any) {
			error = e.message?.includes('403') ? 'Access denied. Admin privileges required.' : (e.message ?? 'Failed to load stats');
			stats = null;
		}
	}

	async function loadUsers() {
		try {
			loading = true;
			users = await api.get<AdminUserRow[]>(`/admin/users?limit=${userLimit}&offset=${userOffset}`);
			error = '';
		} catch (e: any) {
			error = e.message?.includes('403') ? 'Access denied.' : (e.message ?? 'Failed to load users');
		} finally { loading = false; }
	}

	async function loadServers() {
		try {
			loading = true;
			servers = await api.get<AdminServerRow[]>(`/admin/servers?limit=${serverLimit}&offset=${serverOffset}`);
			error = '';
		} catch (e: any) {
			error = e.message ?? 'Failed to load instances';
		} finally { loading = false; }
	}

	async function loadRateLimits() {
		try {
			rateLimits = await api.get<RateLimitConfig>('/admin/rate-limits');
			rlRps = rateLimits.requests_per_second;
			rlBurst = rateLimits.burst_size;
			rlEnabled = rateLimits.enabled;
			rlError = '';
		} catch (e: any) {
			rlError = e.message ?? 'Failed to load rate limits';
		}
	}

	async function saveRateLimits() {
		rlSaving = true;
		rlError = '';
		try {
			rateLimits = await api.put<RateLimitConfig>('/admin/rate-limits', {
				requests_per_second: rlRps,
				burst_size: rlBurst,
				enabled: rlEnabled
			});
			rlRps = rateLimits.requests_per_second;
			rlBurst = rateLimits.burst_size;
		} catch (e: any) {
			rlError = e.message ?? 'Failed to save rate limits';
		} finally { rlSaving = false; }
	}

	async function handleDeleteUser(userId: string) {
		try {
			await api.delete(`/admin/users/${userId}`);
			await loadUsers();
			deleteConfirm = null;
		} catch (e: any) { error = e.message ?? 'Failed to delete user'; }
	}

	async function handleDeleteServer(serverId: string) {
		try {
			await api.delete(`/admin/servers/${serverId}`);
			await loadServers();
			deleteConfirm = null;
		} catch (e: any) { error = e.message ?? 'Failed to delete server'; }
	}

	function switchTab(tab: typeof activeTab) {
		activeTab = tab;
		if (tab === 'instances' && servers.length === 0) loadServers();
		if (tab === 'rate-limits' && rateLimits === null) loadRateLimits();
	}

	function formatBytes(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
		return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
	}

	function openDeleteConfirm(type: 'user' | 'server', id: string, name: string) {
		deleteConfirm = { type, id, name };
	}

	function confirmDelete() {
		if (!deleteConfirm) return;
		if (deleteConfirm.type === 'user') handleDeleteUser(deleteConfirm.id);
		else handleDeleteServer(deleteConfirm.id);
	}
</script>

<div class="min-h-screen bg-gray-900 px-4 py-6 sm:px-6 lg:p-8">
	<div class="max-w-7xl mx-auto">
		<div class="flex flex-wrap items-center gap-2 sm:gap-3 mb-6 sm:mb-8">
			<button onclick={() => history.back()} class="text-gray-400 hover:text-white text-sm">← Back</button>
			<h1 class="text-2xl sm:text-3xl font-semibold text-white">Admin Dashboard</h1>
		</div>

		{#if error}
			<div class="mb-6 px-4 py-3 bg-gray-900/40 border border-gray-700/50 rounded text-gray-300 text-sm">{error}</div>
		{/if}

		<!-- Stats Cards (2 rows: counts + storage) -->
		{#if stats}
			<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3 sm:gap-4 mb-4">
				<div class="bg-gray-800 rounded-lg p-4"><div class="text-gray-400 text-xs uppercase mb-1">Total Users</div><div class="text-2xl font-bold text-white">{stats.total_users}</div></div>
				<div class="bg-gray-800 rounded-lg p-4"><div class="text-gray-400 text-xs uppercase mb-1">Total Servers</div><div class="text-2xl font-bold text-white">{stats.total_servers}</div></div>
				<div class="bg-gray-800 rounded-lg p-4"><div class="text-gray-400 text-xs uppercase mb-1">Total Messages</div><div class="text-2xl font-bold text-white">{stats.total_messages}</div></div>
				<div class="bg-gray-800 rounded-lg p-4"><div class="text-gray-400 text-xs uppercase mb-1">Voice Sessions</div><div class="text-2xl font-bold text-white">{stats.active_voice_sessions}</div></div>
			</div>
			<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3 sm:gap-4 mb-8">
				<div class="bg-gray-800 rounded-lg p-4"><div class="text-gray-400 text-xs uppercase mb-1">DB Size</div><div class="text-2xl font-bold text-white">{formatBytes(stats.db_size_bytes)}</div></div>
				<div class="bg-gray-800 rounded-lg p-4"><div class="text-gray-400 text-xs uppercase mb-1">File Storage</div><div class="text-2xl font-bold text-white">{formatBytes(stats.attachment_storage_bytes)}</div></div>
				<div class="bg-gray-800 rounded-lg p-4"><div class="text-gray-400 text-xs uppercase mb-1">Total Files</div><div class="text-2xl font-bold text-white">{stats.attachment_count}</div></div>
			</div>
		{/if}

		<!-- Tabs -->
		<div class="flex gap-2 mb-6 border-b border-gray-700">
			{#each [['users', `Users (${stats?.total_users ?? 0})`], ['instances', `Instances (${stats?.total_servers ?? 0})`], ['rate-limits', 'Rate Limits']] as [tab, label]}
				<button
					onclick={() => switchTab(tab as typeof activeTab)}
					class="px-4 py-2 text-sm font-medium transition-colors {activeTab === tab ? 'text-gray-400 border-b-2 border-gray-400' : 'text-gray-400 hover:text-gray-300'}"
				>{label}</button>
			{/each}
		</div>

		{#if activeTab === 'users'}
			<UsersTable {users} {loading} totalUsers={stats?.total_users ?? 0} offset={userOffset} limit={userLimit}
				onDelete={openDeleteConfirm} onPrevPage={() => { userOffset = Math.max(0, userOffset - userLimit); loadUsers(); }}
				onNextPage={() => { userOffset += userLimit; loadUsers(); }} />
		{/if}

		{#if activeTab === 'instances'}
			<ServersTable {servers} {loading} totalServers={stats?.total_servers ?? 0} offset={serverOffset} limit={serverLimit}
				onDelete={openDeleteConfirm} onPrevPage={() => { serverOffset = Math.max(0, serverOffset - serverLimit); loadServers(); }}
				onNextPage={() => { serverOffset += serverLimit; loadServers(); }} />
		{/if}

		{#if activeTab === 'rate-limits'}
			<div class="bg-gray-800 rounded-lg p-4 sm:p-6 max-w-md w-full">
				<h2 class="text-white font-medium mb-4">Rate Limit Configuration</h2>
				<p class="text-gray-400 text-sm mb-6">Changes take effect immediately — no restart required.</p>
				{#if rlError}
					<div class="mb-4 px-3 py-2 bg-gray-900/40 border border-gray-700/50 rounded text-gray-300 text-sm">{rlError}</div>
				{/if}
				<div class="space-y-4">
					<label class="block">
						<span class="text-gray-300 text-sm">Requests per second (per IP)</span>
						<input type="number" min="1" bind:value={rlRps} class="mt-1 block w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white text-sm focus:outline-none focus:border-gray-500" />
					</label>
					<label class="block">
						<span class="text-gray-300 text-sm">Burst size (max tokens per IP)</span>
						<input type="number" min="1" bind:value={rlBurst} class="mt-1 block w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white text-sm focus:outline-none focus:border-gray-500" />
					</label>
					<label class="flex items-center gap-3 cursor-pointer">
						<input type="checkbox" bind:checked={rlEnabled} class="w-4 h-4 rounded bg-gray-700 border-gray-500 text-gray-500 focus:ring-gray-500" />
						<span class="text-gray-300 text-sm">Rate limiting enabled</span>
					</label>
					<button
						onclick={saveRateLimits}
						disabled={rlSaving}
						class="w-full py-2 px-4 bg-gray-600 hover:bg-gray-500 disabled:bg-gray-600 text-white rounded text-sm font-medium transition-colors"
					>{rlSaving ? 'Saving...' : 'Save'}</button>
				</div>
			</div>
		{/if}

		<DeleteConfirmModal
			isOpen={deleteConfirm !== null}
			type={deleteConfirm?.type ?? 'user'}
			name={deleteConfirm?.name ?? ''}
			onCancel={() => (deleteConfirm = null)}
			onConfirm={confirmDelete}
		/>
	</div>
</div>
