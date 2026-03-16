<script lang="ts">
	/**
	 * @file Server home page
	 * @purpose Landing page when no server is selected
	 * @depends stores/servers
	 * @version 1.0.0
	 */
	import { servers } from '$lib/stores/servers';
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
				class="px-6 py-3 bg-indigo-600 hover:bg-indigo-700 text-white font-medium rounded-lg transition-colors"
			>
				Create Server
			</button>
			<button
				class="px-6 py-3 bg-gray-700 hover:bg-gray-600 text-white font-medium rounded-lg transition-colors"
			>
				Join Server
			</button>
		</div>

		{#if $servers.length > 0}
			<div class="mt-8">
				<p class="text-gray-400 text-sm">Your servers:</p>
				<ul class="mt-2 space-y-1 text-gray-300">
					{#each $servers as server (server.id)}
						<li class="text-sm">• {server.name}</li>
					{/each}
				</ul>
			</div>
		{/if}
	</div>
</div>
