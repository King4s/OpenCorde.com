<script lang="ts">
	/**
	 * @file DM home page
	 * @purpose Show list of direct message conversations
	 * @depends stores/dms, stores/auth
	 */
	import { browser } from '$app/environment';
	import { dmChannels, dmUnreadCounts, fetchDmChannels, openDm, openFederatedDm } from '$lib/stores/dms';
	import { presenceMap } from '$lib/stores/presence';
	import api from '$lib/api/client';

	let searchQuery = $state('');
	let searchResults = $state<Array<{ id: string; username: string }>>([]);
	let isSearching = $state(false);
	let showSearchResults = $state(false);
	let error = $state('');

	// True when input looks like "username@hostname" (cross-server DM)
	let isFederatedAddress = $derived(
		searchQuery.includes('@') &&
		/^[a-zA-Z0-9._-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/.test(searchQuery.trim())
	);

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
	 * Search for local users. Skipped when address looks like username@server.
	 */
	async function handleSearch() {
		if (!searchQuery.trim() || isFederatedAddress) {
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

	/** Start a DM with a local user by ID */
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

	/** Start a cross-server DM with username@hostname */
	async function startFederatedDm() {
		const address = searchQuery.trim();
		if (!address) return;
		error = '';
		isSearching = true;
		try {
			const dm = await openFederatedDm(address);
			searchQuery = '';
			showSearchResults = false;
			window.location.href = `/@me/dms/${dm.id}`;
		} catch (e: any) {
			error = e.message || 'Could not reach remote server';
		} finally {
			isSearching = false;
		}
	}

	function getInitials(name: string): string {
		// For "alice@server.com", show "alice" initials
		const displayName = name.includes('@') ? name.split('@')[0] : name;
		return displayName.slice(0, 2).toUpperCase();
	}

	function getAvatarColor(userId: string): string {
		const colors = [
			'bg-gray-600',
			'bg-gray-600',
			'bg-gray-600',
			'bg-gray-600',
			'bg-gray-600',
			'bg-gray-600',
			'bg-gray-600',
			'bg-gray-600'
		];
		const hash = userId.split('').reduce((acc, c) => acc + c.charCodeAt(0), 0);
		return colors[hash % colors.length];
	}

	/** For federated DMs, show a badge with the server hostname */
	function getFederationBadge(username: string): string | null {
		if (username.includes('@')) return username.split('@')[1];
		return null;
	}
</script>

<div class="flex-1 flex flex-col bg-gray-900">
	<!-- Header -->
	<div class="h-12 px-4 flex items-center border-b border-gray-800">
		<h1 class="text-lg font-semibold text-white">Direct Messages</h1>
	</div>

	<!-- Search / new DM box -->
	<div class="relative p-4 border-b border-gray-800">
		<div class="relative">
			<input
				type="text"
				bind:value={searchQuery}
				onkeyup={(e) => {
					if (e.key === 'Enter' && isFederatedAddress) startFederatedDm();
					else handleSearch();
				}}
				placeholder="Search users or type username@server.com..."
				class="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded-lg text-white text-sm placeholder-gray-500 focus:outline-none focus:border-gray-500"
			/>
			{#if isSearching}
				<div class="absolute right-3 top-2 text-xs text-gray-500">Connecting...</div>
			{/if}
		</div>

		<!-- Cross-server DM prompt -->
		{#if isFederatedAddress}
			<div class="mt-2 flex items-center gap-2">
				<div class="flex-1 text-xs text-gray-300">
					Start a cross-server DM with <strong>{searchQuery.trim()}</strong>
				</div>
				<button
					onclick={startFederatedDm}
					disabled={isSearching}
					class="px-3 py-1.5 bg-gray-600 hover:bg-gray-500 disabled:opacity-50 text-white text-xs rounded-lg transition-colors"
				>
					Open DM
				</button>
			</div>
		{/if}

		<!-- Local search results dropdown -->
		{#if showSearchResults && !isFederatedAddress && searchResults.length > 0}
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
		{:else if showSearchResults && !isFederatedAddress && searchQuery.trim()}
			<div class="absolute top-16 left-4 right-4 bg-gray-800 border border-gray-700 rounded-lg shadow-lg z-10 px-3 py-2">
				<p class="text-xs text-gray-500">No users found — try username@server.com for cross-server</p>
			</div>
		{/if}

		{#if error}
			<p class="text-gray-400 text-xs mt-2">{error}</p>
		{/if}
	</div>

	<!-- DM list -->
	<div class="flex-1 overflow-y-auto">
		{#if $dmChannels.length === 0}
			<div class="flex flex-col items-center justify-center h-full text-center px-4">
				<div class="text-4xl mb-3">💬</div>
				<p class="text-gray-400">No direct messages yet</p>
				<p class="text-gray-500 text-sm mt-1">Search for a user or type username@server.com for cross-server DMs</p>
			</div>
		{:else}
			<div class="space-y-1 p-2">
				{#each $dmChannels as dm (dm.id)}
					<button
						onclick={() => (window.location.href = `/@me/dms/${dm.id}`)}
						class="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg hover:bg-gray-800 transition-colors text-left"
					>
						<div class="relative flex-shrink-0 w-10 h-10 rounded-full {getAvatarColor(dm.other_user_id)} flex items-center justify-center text-white font-semibold text-sm">
							{getInitials(dm.other_username)}
							{#if $presenceMap.has(dm.other_user_id)}
								<span class="absolute -bottom-0.5 -right-0.5 h-3 w-3 rounded-full bg-gray-500 ring-2 ring-gray-900"></span>
							{/if}
						</div>
						<div class="flex-1 min-w-0">
							<div class="flex items-center gap-2 min-w-0">
								<p class="text-sm font-medium text-white truncate">{dm.other_username}</p>
								{#if ($dmUnreadCounts.get(dm.id) ?? 0) > 0}
									<span class="ml-auto inline-flex items-center justify-center rounded-full bg-gray-500 px-2 py-0.5 text-[11px] font-semibold text-white">
										{$dmUnreadCounts.get(dm.id)}
									</span>
								{/if}
							</div>
							{#if getFederationBadge(dm.other_username)}
								<p class="text-xs text-gray-400">via {getFederationBadge(dm.other_username)}</p>
							{:else}
								<p class="text-xs text-gray-500">Direct message</p>
							{/if}
						</div>
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>
