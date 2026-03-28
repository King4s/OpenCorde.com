<script lang="ts">
	/**
	 * @file User profile popover
	 * @purpose Inline popup showing a user's avatar, roles, bio, and DM/friend actions
	 * @depends api/client, stores/dms, api/types
	 * @version 1.0.0
	 */
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import api from '$lib/api/client';
	import { openDm } from '$lib/stores/dms';
	import type { Role } from '$lib/stores/roles';

	interface PublicProfile {
		id: string;
		username: string;
		avatar_url: string | null;
		status: number;
		bio: string | null;
		status_message: string | null;
	}

	interface Props {
		userId: string;
		serverId: string | null;
		anchorRect: DOMRect;
		onClose: () => void;
	}

	let { userId, serverId, anchorRect, onClose }: Props = $props();

	let profile = $state<PublicProfile | null>(null);
	let memberRoles = $state<Role[]>([]);
	let loading = $state(true);
	let dmLoading = $state(false);
	let popoverEl: HTMLDivElement;

	// Colours for avatar based on user ID (same hash as MessageList)
	const avatarColors = [
		'#5865f2', '#9b59b6', '#e91e8c', '#e74c3c',
		'#e67e22', '#f1c40f', '#2ecc71', '#1abc9c'
	];

	function getAvatarColor(id: string): string {
		const hash = id.split('').reduce((acc, c) => acc + c.charCodeAt(0), 0);
		return avatarColors[hash % avatarColors.length];
	}

	function getInitials(name: string): string {
		return name.slice(0, 2).toUpperCase();
	}

	const statusLabel = ['Online', 'Idle', 'Do Not Disturb', 'Invisible', 'Offline'];
	const statusColor = ['#23a55a', '#f0b232', '#f23f43', '#80848e', '#80848e'];

	// Computed popover position: right of anchor, flip left if too close to right edge
	let style = $derived.by(() => {
		const padding = 8;
		const popoverW = 280;
		const viewW = window.innerWidth;
		let left = anchorRect.right + padding;
		if (left + popoverW > viewW - padding) {
			left = anchorRect.left - popoverW - padding;
		}
		const top = Math.max(padding, Math.min(anchorRect.top, window.innerHeight - 400));
		return `left:${left}px;top:${top}px`;
	});

	onMount(() => {
		// Load profile + roles in parallel
		Promise.all([
			api.get<PublicProfile>(`/users/${userId}`),
			serverId ? api.get<Role[]>(`/servers/${serverId}/members/${userId}/roles`) : Promise.resolve([])
		]).then(([prof, roles]) => {
			profile = prof;
			memberRoles = roles as Role[];
			loading = false;
		}).catch(() => {
			loading = false;
		});

		// Close on click outside
		function handleOutsideClick(e: MouseEvent) {
			if (popoverEl && !popoverEl.contains(e.target as Node)) onClose();
		}
		document.addEventListener('mousedown', handleOutsideClick);
		return () => document.removeEventListener('mousedown', handleOutsideClick);
	});

	async function handleSendMessage() {
		if (!profile) return;
		dmLoading = true;
		try {
			const dm = await openDm(userId);
			onClose();
			goto(`/@me/dms/${dm.id}`);
		} catch {
			dmLoading = false;
		}
	}

	function roleColor(color: number | null): string {
		if (!color) return '#5865f2';
		const r = (color >> 16) & 0xff;
		const g = (color >> 8) & 0xff;
		const b = color & 0xff;
		return `rgb(${r},${g},${b})`;
	}
</script>

<!-- Popover overlay -->
<div
	bind:this={popoverEl}
	class="fixed z-50 w-70 bg-gray-850 border border-gray-700 rounded-lg shadow-2xl overflow-hidden"
	style={style}
	role="dialog"
	aria-modal="true"
	aria-label="User profile"
>
	{#if loading}
		<div class="p-6 flex items-center justify-center">
			<div class="text-gray-500 text-sm">Loading...</div>
		</div>
	{:else if profile}
		<!-- Profile banner / avatar section -->
		<div class="bg-gray-900/50 px-4 pt-4 pb-10 relative">
			<button
				class="absolute top-2 right-2 text-gray-500 hover:text-gray-300 text-lg leading-none"
				onclick={onClose}
				aria-label="Close"
			>×</button>
		</div>

		<!-- Avatar (overlaps banner) -->
		<div class="px-4 -mt-8 mb-2 flex items-end gap-3">
			<div
				class="w-16 h-16 rounded-full flex items-center justify-center text-white font-bold text-lg border-4 border-gray-850 flex-shrink-0"
				style="background-color:{getAvatarColor(profile.id)}"
			>
				{#if profile.avatar_url}
					<img src={profile.avatar_url} alt={profile.username} class="w-full h-full rounded-full object-cover" />
				{:else}
					{getInitials(profile.username)}
				{/if}
			</div>
			<!-- Status dot -->
			<div class="mb-1">
				<span
					class="inline-block w-3 h-3 rounded-full border-2 border-gray-850"
					style="background-color:{statusColor[profile.status] ?? statusColor[4]}"
					title={statusLabel[profile.status] ?? 'Offline'}
				></span>
			</div>
		</div>

		<!-- Name + status message -->
		<div class="px-4 pb-2">
			<div class="font-semibold text-white text-base">{profile.username}</div>
			{#if profile.status_message}
				<div class="text-gray-400 text-xs mt-0.5 truncate">{profile.status_message}</div>
			{/if}
		</div>

		<!-- Divider -->
		<div class="mx-4 border-t border-gray-700/50"></div>

		<!-- Bio -->
		{#if profile.bio}
			<div class="px-4 py-2">
				<div class="text-xs text-gray-500 uppercase font-semibold mb-1">About Me</div>
				<div class="text-gray-300 text-xs leading-relaxed">{profile.bio}</div>
			</div>
		{/if}

		<!-- Roles -->
		{#if memberRoles.length > 0}
			<div class="px-4 py-2">
				<div class="text-xs text-gray-500 uppercase font-semibold mb-1.5">Roles</div>
				<div class="flex flex-wrap gap-1">
					{#each memberRoles as role (role.id)}
						<span
							class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs border"
							style="color:{roleColor(role.color)};border-color:{roleColor(role.color)}40;background-color:{roleColor(role.color)}15"
						>
							<span class="w-2 h-2 rounded-full inline-block" style="background-color:{roleColor(role.color)}"></span>
							{role.name}
						</span>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Actions -->
		<div class="px-4 py-3 border-t border-gray-700/50">
			<button
				onclick={handleSendMessage}
				disabled={dmLoading}
				class="w-full py-1.5 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white text-sm font-medium rounded transition-colors"
			>
				{dmLoading ? 'Opening...' : 'Send Message'}
			</button>
		</div>
	{:else}
		<div class="p-6 text-center text-gray-500 text-sm">Could not load profile.</div>
	{/if}
</div>
