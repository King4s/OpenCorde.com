<script lang="ts">
	/**
	 * @file DM home page
	 * @purpose Show list of direct message conversations
	 * @depends stores/dms, stores/auth
	 */
	import { browser } from '$app/environment';
	import { dmChannels, fetchDmChannels, openDm } from '$lib/stores/dms';
	import api from '$lib/api/client';

	let searchQuery = $state('');
	let searchResults = $state<Array<{ id: string; username: string }>>([]);
	let isSearching = $state(false);
	let showSearchResults = $state(false);
	let error = $state('');

	// Load DM channels on mount
	if (browser) {
		const token = localStorage.getItem('opencorde_token');
		if (!token) {
			window.location.href = '/login';
		} else {
			fetchDmChannels().catch(() => {});
		}
	}

	/**
	 * Search for users to start new DM
	 */
	async function handleSearch() {
		if (!searchQuery.trim()) {
			searchResults = [];
			showSearchResults = false;
			return;
		}

		isSearching = true;
		error = '';
		try {
			const results = await api.get<Array<{ id: string; username: string }>>(
				`/users/search?q=${encodeURIComponent(searchQuery.trim())}`
			);
			searchResults = results;
			showSearchResults = true;
		} catch (e: any) {
			error = e.message || 'Search failed';
			searchResults = [];
		} finally {
			isSearching = false;
		}
	}

	/**
	 * Start a DM with a user
	 */
	async function startDm(userId: string) {
		error = '';
		try {
			const dm = await openDm(userId);
			searchQuery = '';
			searchResults = [];
			showSearchResults = false;
			window.location.href = `/@me/dms/${dm.id}`;
		} catch (e: any) {
			error = e.message || 'Failed to open DM';
		}
	}

	function getInitials(name: string): string {
		return name.slice(0, 2).toUpperCase();
	}

	function getAvatarColor(userId: string): string {
		const colors = [
			'bg-indigo-600',
			'bg-purple-600',
			'bg-pink-600',
			'bg-red-600',
			'bg-orange-600',
			'bg-yellow-600',
			'bg-green-600',
			'bg-teal-600'
		];
		const hash = userId.split('').reduce((acc, c) => acc + c.charCodeAt(0), 0);
		return colors[hash % colors.length];
	}
</script>

<div class="flex-1 flex flex-col bg-gray-900">
	<!-- Header -->
	<div class="h-12 px-4 flex items-center border-b border-gray-800">
		<h1 class="text-lg font-semibold text-white">Direct Messages</h1>
	</div>

	<!-- Search box -->
	<div class="p-4 border-b border-gray-800">
		<div class="relative">
			<input
				type="text"
				bind:value={searchQuery}
				onkeyup={() => handleSearch()}
				placeholder="Search users..."
				class="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded-lg text-white text-sm placeholder-gray-500 focus:outline-none focus:border-indigo-500"
			/>
			{#if isSearching}
				<div class="absolute right-3 top-2 text-xs text-gray-500">Searching...</div>
			{/if}
		</div>

		<!-- Search results dropdown -->
		{#if showSearchResults && searchResults.length > 0}
			<div class="absolute top-16 left-4 right-4 bg-gray-800 border border-gray-700 rounded-lg shadow-lg z-10 max-h-48 overflow-y-auto">
				{#each searchResults as user (user.id)}
					<button
						onclick={() => startDm(user.id)}
						class="w-full text-left px-3 py-2 hover:bg-gray-700 text-sm text-gray-300 border-b border-gray-700/50 last:border-b-0"
					>
						{user.username}
					</button>
				{/each}
			</div>
		{:else if showSearchResults && searchQuery.trim()}
			<div class="absolute top-16 left-4 right-4 bg-gray-800 border border-gray-700 rounded-lg shadow-lg z-10 px-3 py-2">
				<p class="text-xs text-gray-500">No users found</p>
			</div>
		{/if}

		{#if error}
			<p class="text-red-400 text-xs mt-2">{error}</p>
		{/if}
	</div>

	<!-- DM list -->
	<div class="flex-1 overflow-y-auto">
		{#if $dmChannels.length === 0}
			<div class="flex flex-col items-center justify-center h-full text-center px-4">
				<div class="text-4xl mb-3">💬</div>
				<p class="text-gray-400">No direct messages yet</p>
				<p class="text-gray-500 text-sm mt-1">Search for a user above to start a conversation</p>
			</div>
		{:else}
			<div class="space-y-1 p-2">
				{#each $dmChannels as dm (dm.id)}
					<button
						onclick={() => (window.location.href = `/@me/dms/${dm.id}`)}
						class="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg hover:bg-gray-800 transition-colors text-left"
					>
						<div class="flex-shrink-0 w-10 h-10 rounded-full {getAvatarColor(dm.other_user_id)} flex items-center justify-center text-white font-semibold text-sm">
							{getInitials(dm.other_username)}
						</div>
						<div class="flex-1 min-w-0">
							<p class="text-sm font-medium text-white truncate">{dm.other_username}</p>
							<p class="text-xs text-gray-500">Direct message</p>
						</div>
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>
