<script lang="ts">
	import { currentUser } from '$lib/stores/auth';
	import { goto } from '$app/navigation';

	function openProfile() {
		goto('/@me');
	}
</script>

<div class="h-14 border-t border-gray-900 bg-gray-850 px-3 flex items-center justify-between gap-2">
	{#if $currentUser}
		<button
			onclick={openProfile}
			class="flex items-center gap-2 min-w-0 text-left hover:bg-gray-800 rounded px-2 py-1 transition-colors"
			title="Open your profile"
		>
			<div class="w-8 h-8 rounded-full bg-indigo-600 flex items-center justify-center text-white text-xs font-semibold flex-shrink-0">
				{($currentUser.username ?? 'ME').slice(0, 2).toUpperCase()}
			</div>
			<div class="min-w-0">
				<div class="text-sm text-white truncate">{$currentUser.username}</div>
				<div class="text-xs text-gray-400 truncate">{$currentUser.status_message ?? 'Online'}</div>
			</div>
		</button>
	{:else}
		<div class="text-xs text-gray-500">Signed out</div>
	{/if}

	<button
		onclick={openProfile}
		class="text-gray-400 hover:text-white text-xs px-2 py-1 rounded hover:bg-gray-800"
		title="Open DMs"
	>
		@
	</button>
</div>
