<script lang="ts">
	/**
	 * @file App layout — server list sidebar + content area
	 * @purpose Main navigation structure for authenticated users
	 */
	import { browser } from '$app/environment';
	import { servers, fetchServers, selectServer, currentServerId } from '$lib/stores/servers';
	import { initDmListener } from '$lib/stores/dms';
	import ServerIcon from '$lib/components/layout/ServerIcon.svelte';
	import api from '$lib/api/client';

	let { children } = $props();

	if (browser) {
		const token = localStorage.getItem('opencorde_token');
		if (!token) {
			window.location.href = '/login';
		} else {
			fetchServers().catch(() => {});
			initDmListener();
			// Sync currentServerId from URL
			const match = window.location.pathname.match(/\/servers\/([^/]+)/);
			if (match) {
				currentServerId.set(match[1]);
			}
		}
	}

	function handleLogout() {
		localStorage.removeItem('opencorde_token');
		window.location.href = '/login';
	}
</script>

<div class="flex h-screen bg-gray-900">
	<nav class="w-[72px] bg-gray-950 flex flex-col items-center py-3 gap-2 overflow-y-auto border-r border-gray-800">
		<button
			class="w-12 h-12 rounded-2xl bg-gray-700 hover:bg-indigo-600 hover:rounded-xl transition-all flex items-center justify-center text-white font-bold text-sm"
			title="Home"
		>
			OC
		</button>

		<button
			onclick={() => (window.location.href = '/@me')}
			class="w-12 h-12 rounded-2xl bg-gray-700 hover:bg-indigo-600 hover:rounded-xl transition-all flex items-center justify-center text-gray-300 hover:text-white text-xl"
			title="Direct Messages"
		>
			💬
		</button>

		<div class="w-8 h-0.5 bg-gray-700 rounded my-1"></div>

		{#each $servers as server (server.id)}
			<ServerIcon
				name={server.name}
				active={$currentServerId === server.id}
				onclick={() => {
					selectServer(server.id);
					window.location.href = `/servers/${server.id}`;
				}}
			/>
		{/each}

		<button
			class="w-12 h-12 rounded-2xl bg-gray-700 hover:bg-green-600 hover:rounded-xl transition-all flex items-center justify-center text-green-400 hover:text-white text-2xl"
			title="Add Server"
		>
			+
		</button>

		<div class="flex-1"></div>

		<button
			onclick={handleLogout}
			class="w-12 h-12 rounded-2xl bg-gray-700 hover:bg-red-600 hover:rounded-xl transition-all flex items-center justify-center text-gray-400 hover:text-white text-lg"
			title="Logout"
		>
			&#x23CB;
		</button>
	</nav>

	<div class="flex-1 flex">
		{@render children()}
	</div>
</div>
