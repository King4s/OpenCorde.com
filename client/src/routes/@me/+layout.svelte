<script lang="ts">
	/**
	 * @file DM layout — wraps @me pages with sidebar
	 * @purpose Shows server list sidebar + DM content area
	 */
import { browser } from '$app/environment';
import { goto } from '$app/navigation';
import { spaces, fetchSpaces, selectSpace, currentSpaceId } from '$lib/stores/servers';
import { initDmListener, hasAnyDmUnread } from '$lib/stores/dms';
import { initPresenceListener } from '$lib/stores/presence';
import ServerIcon from '$lib/components/layout/ServerIcon.svelte';

	let { children } = $props();

	if (browser) {
		const token=localStorage.getItem('opencorde_token');
		if (!token) {
			window.location.href = '/login';
		} else {
			fetchSpaces().catch(() => {});
			initDmListener();
			initPresenceListener();
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
			onclick={() => goto('/servers')}
			class="w-12 h-12 rounded-2xl bg-gray-700 hover:bg-gray-600 hover:rounded-xl transition-all flex items-center justify-center text-white font-bold text-sm"
			title="Home"
		>
			OC
		</button>

		<button
			onclick={() => goto('/@me')}
			class="relative w-12 h-12 rounded-2xl bg-gray-600 hover:bg-gray-700 hover:rounded-xl transition-all flex items-center justify-center text-white text-xl"
			title="Direct Messages"
		>
			💬
			{#if $hasAnyDmUnread}
				<span class="absolute -top-1 -right-1 h-3 w-3 rounded-full bg-gray-500 ring-2 ring-gray-950"></span>
			{/if}
		</button>

		<a
			href="/@me/friends"
			class="w-12 h-12 rounded-2xl bg-gray-700 hover:bg-gray-600 hover:rounded-xl transition-all flex items-center justify-center text-white text-xl"
			title="Friends"
		>
			👥
		</a>

		<a
			href="/discover"
			class="w-12 h-12 rounded-2xl bg-gray-700 hover:bg-gray-600 hover:rounded-xl transition-all flex items-center justify-center text-white text-xl"
			title="Discover Servers"
		>
			🔭
		</a>

		<div class="w-8 h-0.5 bg-gray-700 rounded my-1"></div>

		{#each $spaces as server (server.id)}
			<ServerIcon
				name={server.name}
				active={$currentSpaceId === server.id}
				onclick={() => {
					selectSpace(server.id);
					window.location.href = `/servers/${server.id}`;
				}}
			/>
		{/each}

		<button
			onclick={() => goto('/servers')}
			class="w-12 h-12 rounded-2xl bg-gray-700 hover:bg-gray-600 hover:rounded-xl transition-all flex items-center justify-center text-gray-400 hover:text-white text-2xl"
			title="Add Space"
		>
			+
		</button>

		<div class="flex-1"></div>

		<button
			onclick={() => { window.location.href = '/settings'; }}
			class="w-12 h-12 rounded-2xl bg-gray-700 hover:bg-gray-600 hover:rounded-xl transition-all flex items-center justify-center text-gray-400 hover:text-white text-lg"
			title="Settings"
		>
			⚙
		</button>

		<button
			onclick={handleLogout}
			class="w-12 h-12 rounded-2xl bg-gray-700 hover:bg-gray-600 hover:rounded-xl transition-all flex items-center justify-center text-gray-400 hover:text-white text-lg"
			title="Logout"
		>
			&#x23CB;
		</button>
	</nav>

	<div class="flex-1 flex">
		{@render children()}
	</div>
</div>
