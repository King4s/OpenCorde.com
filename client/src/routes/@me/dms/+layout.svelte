<script lang="ts">
	/**
	 * @file DM conversation layout
	 * @purpose Wraps DM conversation pages
	 */
	import { dmChannels, fetchDmChannels } from '$lib/stores/dms';
	import { browser } from '$app/environment';

	let { children } = $props();

	if (browser) {
		fetchDmChannels().catch(() => {});
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

	function getCurrentDm() {
		if (!browser) return null;
		const match = window.location.pathname.match(/\/@me\/dms\/([^/]+)/);
		const dmId = match?.[1] ?? '';
		return $dmChannels.find(dm => dm.id === dmId);
	}
</script>

<div class="flex flex-1">
	<!-- DM sidebar -->
	<div class="w-60 bg-gray-800 flex flex-col border-r border-gray-900">
		<!-- Header -->
		<div class="h-12 px-3 flex items-center border-b border-gray-900">
			<h2 class="font-semibold text-white truncate text-sm">
				{getCurrentDm()?.other_username ?? 'Direct Message'}
			</h2>
		</div>

		<!-- DM list -->
		<div class="flex-1 overflow-y-auto p-2 space-y-1">
			{#if $dmChannels.length === 0}
				<p class="text-gray-500 text-xs px-2 py-2">No conversations</p>
			{:else}
				{#each $dmChannels as dm (dm.id)}
					<button
						onclick={() => (window.location.href = `/@me/dms/${dm.id}`)}
						class="w-full flex items-center gap-2 px-2 py-1.5 rounded text-left hover:bg-gray-700 transition-colors"
					>
						<div class="flex-shrink-0 w-8 h-8 rounded-full {getAvatarColor(dm.other_user_id)} flex items-center justify-center text-white font-semibold text-xs">
							{getInitials(dm.other_username)}
						</div>
						<span class="text-xs font-medium text-gray-300 truncate">{dm.other_username}</span>
					</button>
				{/each}
			{/if}
		</div>
	</div>

	<!-- Content -->
	<div class="flex-1 flex flex-col">
		{@render children()}
	</div>
</div>
