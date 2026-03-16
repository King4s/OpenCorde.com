<script lang="ts">
	/**
	 * @file App layout — server list sidebar + content area
	 * @purpose Main navigation structure for authenticated users
	 * @depends stores/servers, stores/auth
	 * @version 1.0.0
	 */
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { isAuthenticated, logout } from '$lib/stores/auth';
	import { servers, fetchServers, selectServer, currentServerId } from '$lib/stores/servers';
	import ServerIcon from '$lib/components/layout/ServerIcon.svelte';

	let { children } = $props();
	let authReady = $state(false);

	onMount(() => {
		const unsub = isAuthenticated.subscribe((v) => {
			if (!v) {
				goto('/login');
			} else {
				authReady = true;
				fetchServers();
			}
		});

		return unsub;
	});

	function handleLogout() {
		logout();
		goto('/login');
	}
</script>

{#if authReady}
	<div class="flex h-screen bg-gray-900">
		<!-- Server sidebar (narrow, Discord-style) -->
		<nav class="w-[72px] bg-gray-950 flex flex-col items-center py-3 gap-2 overflow-y-auto border-r border-gray-900">
			<!-- Home button -->
			<button
				class="w-12 h-12 rounded-2xl bg-gray-700 hover:bg-indigo-600 hover:rounded-xl transition-all flex items-center justify-center text-white font-bold text-sm"
				onclick={() => goto('/servers')}
				title="Home"
			>
				OC
			</button>

			<div class="w-8 h-0.5 bg-gray-700 rounded my-1"></div>

			<!-- Server list -->
			{#each $servers as server (server.id)}
				<ServerIcon
					name={server.name}
					active={$currentServerId === server.id}
					onclick={() => selectServer(server.id)}
				/>
			{/each}

			<!-- Add server button -->
			<button
				class="w-12 h-12 rounded-2xl bg-gray-700 hover:bg-green-600 hover:rounded-xl transition-all flex items-center justify-center text-green-400 hover:text-white text-2xl"
				title="Add Server"
			>
				+
			</button>

			<div class="flex-1"></div>

			<!-- Logout button -->
			<button
				onclick={handleLogout}
				class="w-12 h-12 rounded-2xl bg-gray-700 hover:bg-red-600 hover:rounded-xl transition-all flex items-center justify-center text-gray-400 hover:text-white text-lg"
				title="Logout"
			>
				⎋
			</button>
		</nav>

		<!-- Main content area -->
		<div class="flex-1 flex">
			{@render children()}
		</div>
	</div>
{/if}
