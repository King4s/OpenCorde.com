<script lang="ts">
	/**
	 * @file Member list sidebar component
	 * @purpose Shows all members in the current server
	 */
	import type { Member } from '$lib/api/types';

	interface Props {
		members: Member[];
		loading: boolean;
	}

	let { members, loading }: Props = $props();

	function getInitials(name: string): string {
		return name.slice(0, 2).toUpperCase();
	}

	function getAvatarColor(userId: string): string {
		const colors = [
			'bg-indigo-600', 'bg-purple-600', 'bg-pink-600', 'bg-red-600',
			'bg-orange-600', 'bg-yellow-600', 'bg-green-600', 'bg-teal-600'
		];
		const hash = userId.split('').reduce((acc, c) => acc + c.charCodeAt(0), 0);
		return colors[hash % colors.length];
	}
</script>

<div class="w-48 bg-gray-800 flex flex-col border-l border-gray-900">
	<div class="h-12 px-3 flex items-center border-b border-gray-900">
		<h3 class="text-xs font-semibold text-gray-400 uppercase">Members — {members.length}</h3>
	</div>

	<div class="flex-1 overflow-y-auto p-2 space-y-0.5">
		{#if loading}
			<p class="text-gray-500 text-xs px-2">Loading...</p>
		{:else if members.length === 0}
			<p class="text-gray-500 text-xs px-2">No members</p>
		{:else}
			{#each members as member (member.user_id)}
				<div class="flex items-center gap-2 px-2 py-1 rounded hover:bg-gray-700/50 group">
					<div class="w-7 h-7 rounded-full {getAvatarColor(member.user_id)} flex items-center justify-center text-white text-xs font-semibold flex-shrink-0">
						{getInitials(member.nickname ?? member.username)}
					</div>
					<span class="text-gray-300 text-sm truncate group-hover:text-white transition-colors">
						{member.nickname ?? member.username}
					</span>
				</div>
			{/each}
		{/if}
	</div>
</div>
