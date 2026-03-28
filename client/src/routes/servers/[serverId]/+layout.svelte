<script lang="ts">
	/**
	 * @file Server layout — channel sidebar + content
	 * @purpose Shows channels, create channel, invite link, channel context menu
	 */
	import { browser } from '$app/environment';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { get } from 'svelte/store';
	import {
		fetchChannels,
		currentChannelId,
		currentChannel
	} from '$lib/stores/channels';
	import { currentServer } from '$lib/stores/servers';
	import { currentUser } from '$lib/stores/auth';
	import { currentVoiceChannelId } from '$lib/stores/voice';
	import { initUnreadListener, loadReadStates } from '$lib/stores/unread';
	import { presenceMap, initPresenceListener } from '$lib/stores/presence';
	import api from '$lib/api/client';
	import VoicePanel from '$lib/components/voice/VoicePanel.svelte';
	import StagePanel from '$lib/components/voice/StagePanel.svelte';
	import MemberList from '$lib/components/layout/MemberList.svelte';
	import ChannelSidebar from '$lib/components/layout/ChannelSidebar.svelte';
	import WebhookManager from '$lib/components/modals/WebhookManager.svelte';
	import { members, membersLoading, fetchMembers, initMemberListeners } from '$lib/stores/members';
	import { initChannelListeners } from '$lib/stores/channels';
	import { initRoleListeners } from '$lib/stores/roles';
	import { initServerListeners } from '$lib/stores/servers';
	import UserPanel from '$lib/components/layout/UserPanel.svelte';
	import QuickSwitcher from '$lib/components/modals/QuickSwitcher.svelte';

	let { children } = $props();
	let serverId = $state('');

	// Modals
	let showCreateChannel = $state(false);
	let showQuickSwitcher = $state(false);
	let showInvite = $state(false);
	let inviteCode = $state('');
	let inviteLoading = $state(false);
	let error = $state('');

	// Webhook manager
	let webhookChannelId = $state<string | null>(null);

	if (browser) {
		const match = window.location.pathname.match(/\/servers\/([^/]+)/);
		serverId = match?.[1] ?? '';
		if (serverId) {
			fetchChannels(serverId).catch(() => {});
			fetchMembers(serverId).catch(() => {});
			initUnreadListener();
			initPresenceListener();
			initChannelListeners();
			initRoleListeners();
			initMemberListeners();
			initServerListeners();
			loadReadStates().catch(() => {});
		}
	}


	onMount(() => {
		function handleKeydown(e: KeyboardEvent) {
			// Ctrl+K — Quick switcher
			if (e.ctrlKey && e.key === 'k') {
				e.preventDefault();
				showQuickSwitcher = !showQuickSwitcher;
				return;
			}
			if (!e.altKey) return;
			if (e.key !== 'ArrowUp' && e.key !== 'ArrowDown') return;
			e.preventDefault();
			const sid = serverId;
			if (!sid) return;
			const allChannels = get(channels);
			const textChs = allChannels
				.filter(c => c.channel_type === 0)
				.sort((a, b) => a.position - b.position);
			if (textChs.length === 0) return;
			const curId = get(currentChannelId);
			const idx = textChs.findIndex(c => c.id === curId);
			let nextIdx = e.key === 'ArrowUp' ? idx - 1 : idx + 1;
			nextIdx = Math.max(0, Math.min(nextIdx, textChs.length - 1));
			const next = textChs[nextIdx];
			if (next && next.id !== curId) {
				goto('/servers/' + sid + '/channels/' + next.id);
			}
		}
		window.addEventListener('keydown', handleKeydown);
		return () => window.removeEventListener('keydown', handleKeydown);
	});

	async function handleCreateInvite() {
		if (!serverId) return;
		inviteLoading = true;
		try {
			const res = await api.post<{ code: string }>(`/servers/${serverId}/invites`, {});
			inviteCode = `https://opencorde.com/invite/${res.code}`;
		} catch (e: any) {
			error = e.message || 'Failed to create invite';
		} finally {
			inviteLoading = false;
		}
	}

	function copyInvite() {
		navigator.clipboard.writeText(inviteCode);
	}

</script>

<div class="flex flex-1">
	<div class="w-60 bg-gray-800 flex flex-col">
		<!-- Server header with actions -->
		<div class="h-12 px-3 flex items-center justify-between border-b border-gray-900">
			<h2 class="font-semibold text-white truncate text-sm">
				{$currentServer?.name ?? 'Server'}
			</h2>
			<div class="flex gap-1">
				<button
					onclick={() => { showInvite = !showInvite; showCreateChannel = false; }}
					class="w-6 h-6 rounded flex items-center justify-center text-gray-400 hover:text-white hover:bg-gray-700 text-xs"
					title="Create Invite"
				>+&#x1F517;</button>
				<button
					onclick={() => { showCreateChannel = !showCreateChannel; showInvite = false; }}
					class="w-6 h-6 rounded flex items-center justify-center text-gray-400 hover:text-white hover:bg-gray-700 text-xs"
					title="Create Channel"
				>+#</button>
				{#if $currentUser?.id === $currentServer?.owner_id}
					<button
						onclick={() => { window.location.href = `/servers/${serverId}/settings`; }}
						class="w-6 h-6 rounded flex items-center justify-center text-gray-400 hover:text-white hover:bg-gray-700 text-xs"
						title="Server Settings"
					>⚙</button>
				{/if}
			</div>
		</div>

		<!-- Invite form -->
		{#if showInvite}
			<div class="p-2 bg-gray-750 border-b border-gray-900">
				{#if inviteCode}
					<div class="flex gap-1">
						<input type="text" value={inviteCode} readonly
							class="flex-1 px-2 py-1 bg-gray-900 border border-gray-700 rounded text-white text-xs" />
						<button onclick={copyInvite}
							class="px-2 py-1 bg-indigo-600 text-white text-xs rounded">Copy</button>
					</div>
				{:else}
					<button onclick={handleCreateInvite} disabled={inviteLoading}
						class="w-full text-xs py-1 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white rounded">
						{inviteLoading ? 'Creating...' : 'Generate Invite Link'}
					</button>
				{/if}
			</div>
		{/if}

		<!-- Channel sidebar -->
		<ChannelSidebar
			{serverId}
			{showCreateChannel}
			onCreateChannelToggle={() => showCreateChannel = false}
			onWebhookOpen={(channelId) => webhookChannelId = channelId}
		/>

		<VoicePanel />
		<UserPanel />
		<UserPanel />
	{#if $currentChannel?.channel_type === 3}
		<StagePanel
			channelId={$currentChannelId || ''}
			{serverId}
			isOwner={$currentUser?.id === $currentServer?.owner_id}
		/>
	{/if}
	</div>

	<div class="flex-1 flex flex-col">
		{@render children()}
	</div>

	<MemberList
		members={$members}
		loading={$membersLoading}
		serverId={serverId}
		isOwner={$currentUser?.id === $currentServer?.owner_id}
		onlineUserIds={new Set($presenceMap.keys())}
	/>
</div>


{#if webhookChannelId}
	<WebhookManager channelId={webhookChannelId} onClose={() => webhookChannelId = null} />
{/if}

{#if showQuickSwitcher}
	<QuickSwitcher onClose={() => showQuickSwitcher = false} />
{/if}
