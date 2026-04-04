<script lang="ts">
	/**
	 * @file App layout — server list sidebar + content area
	 * @purpose Main navigation structure for authenticated users
	 */
	import { browser } from '$app/environment';
	import { spaces, fetchSpaces, selectSpace, currentSpaceId } from '$lib/stores/servers';
	import { serverHasUnread } from '$lib/stores/unread';
	import { hasAnyDmUnread, initDmListener } from '$lib/stores/dms';
	import ServerIcon from '$lib/components/layout/ServerIcon.svelte';
	import api from '$lib/api/client';

	let { children } = $props();

	if (browser) {
		const token = localStorage.getItem('opencorde_token');
		if (!token) {
			window.location.href = '/login';
		} else {
			fetchSpaces().catch(() => {});
			initDmListener();
			// Sync currentSpaceId from URL
			const match = window.location.pathname.match(/\/servers\/([^/]+)/);
			if (match) {
				currentSpaceId.set(match[1]);
			}
		}
	}

	function handleLogout() {
		localStorage.removeItem('opencorde_token');
		window.location.href = '/login';
	}
</script>

<div class="flex h-screen bg-gray-900">
	<nav
		aria-label="Main navigation"
		class="w-[72px] bg-gray-950 flex flex-col items-center py-3 gap-2 overflow-y-auto border-r border-gray-800"
	>
		<!-- Home / spaces overview -->
		<button
			onclick={() => (window.location.href = '/servers')}
			class="w-12 h-12 rounded-2xl bg-gray-700 hover:bg-gray-600 hover:rounded-xl transition-all flex items-center justify-center text-white font-bold text-sm"
			title="Home"
			aria-label="Go to spaces overview"
		>
			OC
		</button>

		<!-- Direct messages -->
		<button
			onclick={() => (window.location.href = '/@me')}
			class="relative w-12 h-12 rounded-2xl bg-gray-700 hover:bg-gray-600 hover:rounded-xl transition-all flex items-center justify-center text-gray-300 hover:text-white text-xl"
			title="Direct Messages"
			aria-label="Direct Messages"
		>
			<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
			</svg>
			{#if $hasAnyDmUnread}
				<span class="absolute -top-1 -right-1 h-3 w-3 rounded-full bg-indigo-500 ring-2 ring-gray-950" aria-label="Unread messages"></span>
			{/if}
		</button>

		<!-- Divider between utility and spaces -->
		<div class="w-8 h-0.5 bg-gray-700 rounded my-1" role="separator"></div>

		<!-- Space icons -->
		{#each $spaces as server (server.id)}
			<ServerIcon
				name={server.name}
				active={$currentSpaceId === server.id}
				hasUnread={$serverHasUnread.has(server.id)}
				onclick={() => {
					selectSpace(server.id);
					window.location.href = `/servers/${server.id}`;
				}}
			/>
		{/each}

		<!-- Add a new space -->
		<button
			onclick={() => (window.location.href = '/servers')}
			class="w-12 h-12 rounded-2xl bg-gray-700 hover:bg-gray-600 hover:rounded-xl transition-all flex items-center justify-center text-gray-400 hover:text-white text-2xl"
			title="Add or join a space"
			aria-label="Add or join a space"
		>
			+
		</button>

		<div class="flex-1"></div>

		<!-- Log out -->
		<button
			onclick={handleLogout}
			class="w-12 h-12 rounded-2xl bg-gray-700 hover:bg-red-700/60 hover:rounded-xl transition-all flex items-center justify-center text-gray-400 hover:text-white"
			title="Log out"
			aria-label="Log out"
		>
			<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
			</svg>
		</button>
	</nav>

	<div class="flex-1 flex">
		{@render children()}
	</div>
</div>
