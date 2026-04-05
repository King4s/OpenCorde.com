<script lang="ts">
	/**
	 * @file Member list sidebar component
	 * @purpose Shows all members in the current server with online status and right-click context menu
	 */
	import type { Member } from '$lib/api/types';
	import MemberContextMenu from '$lib/components/modals/MemberContextMenu.svelte';

	interface Props {
		members: Member[];
		loading: boolean;
		spaceId: string;
		isOwner: boolean;
		onlineUserIds?: Set<string>;
	}

	interface ContextMenuState {
		visible: boolean;
		x: number;
		y: number;
		userId: string;
		username: string;
	}

	let { members, loading, spaceId, isOwner, onlineUserIds = new Set() }: Props = $props();
	let contextMenu = $state<ContextMenuState>({
		visible: false,
		x: 0,
		y: 0,
		userId: '',
		username: ''
	});

	function getInitials(name: string): string {
		return name.slice(0, 2).toUpperCase();
	}

	function getAvatarColor(userId: string): string {
		const colors = [
			'bg-gray-600', 'bg-gray-600', 'bg-gray-600', 'bg-gray-600',
			'bg-gray-600', 'bg-gray-600', 'bg-gray-600', 'bg-gray-600'
		];
		const hash = userId.split('').reduce((acc, c) => acc + c.charCodeAt(0), 0);
		return colors[hash % colors.length];
	}

	function handleMemberContextMenu(e: MouseEvent, member: Member) {
		e.preventDefault();
		contextMenu.visible = true;
		contextMenu.x = e.clientX;
		contextMenu.y = e.clientY;
		contextMenu.userId = member.user_id;
		contextMenu.username = member.nickname ?? member.username;
	}

	function closeContextMenu() {
		contextMenu.visible = false;
	}

	// Partition members: online first, then offline
	let onlineMembers = $derived(members.filter(m => onlineUserIds.has(m.user_id)));
	let offlineMembers = $derived(members.filter(m => !onlineUserIds.has(m.user_id)));
</script>

<div class="w-full min-w-0 bg-gray-800 flex flex-col flex-shrink-0 overflow-auto border-l border-gray-900" style="min-width: 12rem; max-width: var(--shell-member-width);">
	<div class="h-10 px-2.5 sm:h-12 sm:px-3 flex items-center border-b border-gray-900">
		<h3 class="text-[10px] font-semibold text-gray-400 uppercase sm:text-xs">Members — {members.length}</h3>
	</div>

	<div class="flex-1 overflow-y-auto p-1.5 sm:p-2 space-y-0.5">
		{#if loading}
			<p class="text-gray-500 text-xs px-2">Loading...</p>
		{:else if members.length === 0}
			<p class="text-gray-500 text-xs px-2">No members</p>
		{:else}
			{#if onlineMembers.length > 0}
				<p class="text-xs font-semibold text-gray-500 uppercase px-2 pt-1 pb-0.5">Online — {onlineMembers.length}</p>
				{#each onlineMembers as member (member.user_id)}
					<div
						role="button"
						tabindex="0"
						class="flex items-center gap-2 px-1.5 py-1 rounded hover:bg-gray-700/50 group cursor-context-menu sm:px-2"
						oncontextmenu={(e) => handleMemberContextMenu(e, member)}
					>
						<div class="relative flex-shrink-0">
							<div class="h-6 w-6 rounded-full {getAvatarColor(member.user_id)} flex items-center justify-center text-white text-[10px] font-semibold sm:h-7 sm:w-7 sm:text-xs">
								{getInitials(member.nickname ?? member.username)}
							</div>
							<!-- Online indicator dot -->
							<span class="absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 rounded-full bg-gray-500 border-2 border-gray-800"></span>
						</div>
						<span class="min-w-0 text-gray-200 text-[13px] truncate group-hover:text-white transition-colors sm:text-sm">
							{member.nickname ?? member.username}
						</span>
					</div>
				{/each}
			{/if}

			{#if offlineMembers.length > 0}
				<p class="text-xs font-semibold text-gray-500 uppercase px-2 pt-2 pb-0.5">Offline — {offlineMembers.length}</p>
				{#each offlineMembers as member (member.user_id)}
					<div
						role="button"
						tabindex="0"
						class="flex items-center gap-2 px-2 py-1 rounded hover:bg-gray-700/50 group cursor-context-menu opacity-60"
						oncontextmenu={(e) => handleMemberContextMenu(e, member)}
					>
						<div class="relative flex-shrink-0">
							<div class="h-6 w-6 rounded-full {getAvatarColor(member.user_id)} flex items-center justify-center text-white text-[10px] font-semibold sm:h-7 sm:w-7 sm:text-xs">
								{getInitials(member.nickname ?? member.username)}
							</div>
						</div>
						<span class="min-w-0 text-gray-400 text-[13px] truncate group-hover:text-gray-200 transition-colors sm:text-sm">
							{member.nickname ?? member.username}
						</span>
					</div>
				{/each}
			{/if}
		{/if}
	</div>
</div>

{#if contextMenu.visible}
	<MemberContextMenu
		userId={contextMenu.userId}
		username={contextMenu.username}
		spaceId={spaceId}
		isOwner={isOwner}
		x={contextMenu.x}
		y={contextMenu.y}
		onClose={closeContextMenu}
	/>
{/if}
