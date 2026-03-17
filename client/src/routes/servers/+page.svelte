<script lang="ts">
	/**
	 * @file Server home page
	 * @purpose Landing page — create server, join via invite
	 * @depends stores/servers
	 */
	import { goto } from '$app/navigation';
	import { servers, createServer, selectServer } from '$lib/stores/servers';
	import api from '$lib/api/client';

	let showCreateModal = $state(false);
	let showJoinModal = $state(false);
	let serverName = $state('');
	let serverDesc = $state('');
	let inviteCode = $state('');
	let error = $state('');
	let loading = $state(false);

	async function handleCreate() {
		if (!serverName.trim()) return;
		error = '';
		loading = true;
		try {
			const server = await createServer(serverName.trim(), serverDesc.trim() || undefined);
			showCreateModal = false;
			serverName = '';
			serverDesc = '';
			window.location.href = '/servers';
		} catch (e: any) {
			error = e.message || 'Failed to create server';
		} finally {
			loading = false;
		}
	}

	async function handleJoin() {
		if (!inviteCode.trim()) return;
		error = '';
		loading = true;
		try {
			await api.post(`/invites/${inviteCode.trim()}/join`);
			showJoinModal = false;
			inviteCode = '';
			window.location.href = '/servers';
		} catch (e: any) {
			error = e.message || 'Invalid invite code';
		} finally {
			loading = false;
		}
	}

	function handleServerClick(id: string) {
		selectServer(id);
		window.location.href = `/servers/${id}/channels/0`;
	}
</script>

<div class="flex-1 flex flex-col items-center justify-center bg-gray-900">
	<div class="text-center">
		<div class="text-6xl mb-4">🎙️</div>
		<h1 class="text-3xl font-bold text-white mb-2">Welcome to OpenCorde</h1>
		<p class="text-gray-400 mb-8">
			{#if $servers.length === 0}
				You haven't joined any servers yet. Create or join one to get started!
			{:else}
				Select a server from the sidebar to begin
			{/if}
		</p>

		<div class="flex gap-4 justify-center">
			<button
				onclick={() => { showCreateModal = true; showJoinModal = false; error = ''; }}
				class="px-6 py-3 bg-indigo-600 hover:bg-indigo-700 text-white font-medium rounded-lg transition-colors"
			>
				Create Server
			</button>
			<button
				onclick={() => { showJoinModal = true; showCreateModal = false; error = ''; }}
				class="px-6 py-3 bg-gray-700 hover:bg-gray-600 text-white font-medium rounded-lg transition-colors"
			>
				Join Server
			</button>
		</div>

		{#if showCreateModal}
			<div class="mt-6 w-full max-w-sm mx-auto bg-gray-800 p-6 rounded-lg text-left">
				<h2 class="text-lg font-semibold text-white mb-4">Create a Server</h2>
				{#if error}<p class="text-red-400 text-sm mb-3">{error}</p>{/if}
				<input
					type="text"
					bind:value={serverName}
					placeholder="Server name"
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 mb-3"
				/>
				<input
					type="text"
					bind:value={serverDesc}
					placeholder="Description (optional)"
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 mb-4"
				/>
				<div class="flex gap-2">
					<button
						onclick={handleCreate}
						disabled={loading || !serverName.trim()}
						class="flex-1 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white rounded transition-colors"
					>
						{loading ? 'Creating...' : 'Create'}
					</button>
					<button
						onclick={() => showCreateModal = false}
						class="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded transition-colors"
					>
						Cancel
					</button>
				</div>
			</div>
		{/if}

		{#if showJoinModal}
			<div class="mt-6 w-full max-w-sm mx-auto bg-gray-800 p-6 rounded-lg text-left">
				<h2 class="text-lg font-semibold text-white mb-4">Join a Server</h2>
				{#if error}<p class="text-red-400 text-sm mb-3">{error}</p>{/if}
				<input
					type="text"
					bind:value={inviteCode}
					placeholder="Invite code"
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 mb-4"
				/>
				<div class="flex gap-2">
					<button
						onclick={handleJoin}
						disabled={loading || !inviteCode.trim()}
						class="flex-1 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white rounded transition-colors"
					>
						{loading ? 'Joining...' : 'Join'}
					</button>
					<button
						onclick={() => showJoinModal = false}
						class="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded transition-colors"
					>
						Cancel
					</button>
				</div>
			</div>
		{/if}

		{#if $servers.length > 0}
			<div class="mt-8">
				<p class="text-gray-400 text-sm mb-2">Your servers:</p>
				{#each $servers as server (server.id)}
					<button
						onclick={() => handleServerClick(server.id)}
						class="block w-full text-left px-4 py-2 rounded hover:bg-gray-800 text-gray-300 text-sm transition-colors"
					>
						{server.name}
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>
