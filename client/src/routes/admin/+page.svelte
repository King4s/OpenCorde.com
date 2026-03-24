<script lang="ts">
	/**
	 * @file Admin Dashboard
	 * @purpose Manage instance resources (users, servers, statistics)
	 */
	import { currentUser } from '$lib/stores/auth';
	import api from '$lib/api/client';
	import type { InstanceStats, AdminUserRow, AdminServerRow } from '$lib/api/types';
	import UsersTable from './UsersTable.svelte';
	import ServersTable from './ServersTable.svelte';
	import DeleteConfirmModal from './DeleteConfirmModal.svelte';

	let stats = $state<InstanceStats | null>(null);
	let users = $state<AdminUserRow[]>([]);
	let servers = $state<AdminServerRow[]>([]);
	let activeTab = $state<'users' | 'servers'>('users');
	let loading = $state(false);
	let error = $state('');
	let userLimit = $state(50);
	let userOffset = $state(0);
	let serverLimit = $state(50);
	let serverOffset = $state(0);
	let deleteConfirm = $state<{
		type: 'user' | 'server';
		id: string;
		name: string;
	} | null>(null);

	// Load stats and users on mount
	$effect(() => {
		loadStats();
		loadUsers();
	});

	async function loadStats() {
		try {
			stats = await api.get<InstanceStats>('/admin/stats');
			error = '';
		} catch (e: any) {
			if (e.message?.includes('403')) {
				error = 'Access denied. Admin privileges required.';
			} else {
				error = e.message ?? 'Failed to load stats';
			}
			stats = null;
		}
	}

	async function loadUsers() {
		try {
			loading = true;
			users = await api.get<AdminUserRow[]>(
				`/admin/users?limit=${userLimit}&offset=${userOffset}`
			);
			error = '';
		} catch (e: any) {
			if (e.message?.includes('403')) {
				error = 'Access denied. Admin privileges required.';
			} else {
				error = e.message ?? 'Failed to load users';
			}
		} finally {
			loading = false;
		}
	}

	async function loadServers() {
		try {
			loading = true;
			servers = await api.get<AdminServerRow[]>(
				`/admin/servers?limit=${serverLimit}&offset=${serverOffset}`
			);
			error = '';
		} catch (e: any) {
			if (e.message?.includes('403')) {
				error = 'Access denied. Admin privileges required.';
			} else {
				error = e.message ?? 'Failed to load servers';
			}
		} finally {
			loading = false;
		}
	}

	async function handleDeleteUser(userId: string, username: string) {
		try {
			await api.delete(`/admin/users/${userId}`);
			await loadUsers();
			deleteConfirm = null;
		} catch (e: any) {
			error = e.message ?? 'Failed to delete user';
		}
	}

	async function handleDeleteServer(serverId: string, name: string) {
		try {
			await api.delete(`/admin/servers/${serverId}`);
			await loadServers();
			deleteConfirm = null;
		} catch (e: any) {
			error = e.message ?? 'Failed to delete server';
		}
	}

	function switchTab(tab: 'users' | 'servers') {
		activeTab = tab;
		if (tab === 'servers' && servers.length === 0) {
			loadServers();
		}
	}

	function nextUserPage() {
		userOffset += userLimit;
		loadUsers();
	}

	function prevUserPage() {
		userOffset = Math.max(0, userOffset - userLimit);
		loadUsers();
	}

	function nextServerPage() {
		serverOffset += serverLimit;
		loadServers();
	}

	function prevServerPage() {
		serverOffset = Math.max(0, serverOffset - serverLimit);
		loadServers();
	}

	function openDeleteConfirm(type: 'user' | 'server', id: string, name: string) {
		deleteConfirm = { type, id, name };
	}

	function confirmDelete() {
		if (!deleteConfirm) return;
		if (deleteConfirm.type === 'user') {
			handleDeleteUser(deleteConfirm.id, deleteConfirm.name);
		} else {
			handleDeleteServer(deleteConfirm.id, deleteConfirm.name);
		}
	}
</script>

<div class="min-h-screen bg-gray-900 p-8">
	<div class="max-w-7xl mx-auto">
		<!-- Header -->
		<div class="flex items-center gap-3 mb-8">
			<button onclick={() => history.back()} class="text-gray-400 hover:text-white text-sm"
				>← Back</button
			>
			<h1 class="text-3xl font-semibold text-white">Admin Dashboard</h1>
		</div>

		<!-- Error Message -->
		{#if error}
			<div class="mb-6 px-4 py-3 bg-red-900/40 border border-red-700/50 rounded text-red-300 text-sm">
				{error}
			</div>
		{/if}

		<!-- Stats Cards -->
		{#if stats}
			<div class="grid grid-cols-2 md:grid-cols-5 gap-4 mb-8">
				<div class="bg-gray-800 rounded-lg p-4">
					<div class="text-gray-400 text-xs uppercase mb-1">Total Users</div>
					<div class="text-2xl font-bold text-white">{stats.total_users}</div>
				</div>
				<div class="bg-gray-800 rounded-lg p-4">
					<div class="text-gray-400 text-xs uppercase mb-1">Total Servers</div>
					<div class="text-2xl font-bold text-white">{stats.total_servers}</div>
				</div>
				<div class="bg-gray-800 rounded-lg p-4">
					<div class="text-gray-400 text-xs uppercase mb-1">Total Messages</div>
					<div class="text-2xl font-bold text-white">{stats.total_messages}</div>
				</div>
				<div class="bg-gray-800 rounded-lg p-4">
					<div class="text-gray-400 text-xs uppercase mb-1">Total Channels</div>
					<div class="text-2xl font-bold text-white">{stats.total_channels}</div>
				</div>
				<div class="bg-gray-800 rounded-lg p-4">
					<div class="text-gray-400 text-xs uppercase mb-1">Voice Sessions</div>
					<div class="text-2xl font-bold text-white">{stats.active_voice_sessions}</div>
				</div>
			</div>
		{/if}

		<!-- Tabs -->
		<div class="flex gap-2 mb-6 border-b border-gray-700">
			<button
				onclick={() => switchTab('users')}
				class="px-4 py-2 text-sm font-medium transition-colors {activeTab === 'users'
					? 'text-indigo-400 border-b-2 border-indigo-400'
					: 'text-gray-400 hover:text-gray-300'}"
			>
				Users ({stats?.total_users ?? 0})
			</button>
			<button
				onclick={() => switchTab('servers')}
				class="px-4 py-2 text-sm font-medium transition-colors {activeTab === 'servers'
					? 'text-indigo-400 border-b-2 border-indigo-400'
					: 'text-gray-400 hover:text-gray-300'}"
			>
				Servers ({stats?.total_servers ?? 0})
			</button>
		</div>

		<!-- Users Tab -->
		{#if activeTab === 'users'}
			<UsersTable
				{users}
				{loading}
				totalUsers={stats?.total_users ?? 0}
				offset={userOffset}
				limit={userLimit}
				onDelete={openDeleteConfirm}
				onPrevPage={prevUserPage}
				onNextPage={nextUserPage}
			/>
		{/if}

		<!-- Servers Tab -->
		{#if activeTab === 'servers'}
			<ServersTable
				{servers}
				{loading}
				totalServers={stats?.total_servers ?? 0}
				offset={serverOffset}
				limit={serverLimit}
				onDelete={openDeleteConfirm}
				onPrevPage={prevServerPage}
				onNextPage={nextServerPage}
			/>
		{/if}

		<!-- Delete Confirmation Modal -->
		<DeleteConfirmModal
			isOpen={deleteConfirm !== null}
			type={deleteConfirm?.type ?? 'user'}
			name={deleteConfirm?.name ?? ''}
			onCancel={() => (deleteConfirm = null)}
			onConfirm={confirmDelete}
		/>
	</div>
</div>
