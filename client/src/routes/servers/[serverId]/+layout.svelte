<script lang="ts">
	/**
	 * @file Space layout — channel sidebar + content
	 * @purpose Shows channels, create channel, invite link, channel context menu
	 */
	import { browser } from '$app/environment';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { get } from 'svelte/store';
	import {
		fetchChannels,
		channels,
		currentChannelId,
		currentChannel
	} from '$lib/stores/channels';
	import { currentSpace } from '$lib/stores/servers';
	import { currentUser } from '$lib/stores/auth';
	import { currentVoiceChannelId } from '$lib/stores/voice';
	import { initUnreadListener, loadReadStates } from '$lib/stores/unread';
	import { presenceMap, initPresenceListener } from '$lib/stores/presence';
	import api from '$lib/api/client';
	import VoicePanel from '$lib/components/voice/VoicePanel.svelte';
	import VoiceStage from '$lib/components/voice/VoiceStage.svelte';
	import SoundboardPanel from '$lib/components/voice/SoundboardPanel.svelte';
	import StagePanel from '$lib/components/voice/StagePanel.svelte';
	import OnboardingModal from '$lib/components/modals/OnboardingModal.svelte';
	import MemberList from '$lib/components/layout/MemberList.svelte';
	import ChannelSidebar from '$lib/components/layout/ChannelSidebar.svelte';
	import WebhookManager from '$lib/components/modals/WebhookManager.svelte';
	import { members, membersLoading, fetchMembers, initMemberListeners } from '$lib/stores/members';
	import { initChannelListeners } from '$lib/stores/channels';
	import { initRoleListeners } from '$lib/stores/roles';
	import { initSpaceListeners } from '$lib/stores/servers';
	import UserPanel from '$lib/components/layout/UserPanel.svelte';
	import QuickSwitcher from '$lib/components/modals/QuickSwitcher.svelte';
	import { edgeResize } from '$lib/actions/edgeResize';

	let { children } = $props();
	let spaceId = $state('');

	// Modals
	let showCreateChannel = $state(false);
	let showQuickSwitcher = $state(false);
	let showInvite = $state(false);
	let inviteCode = $state('');
	let inviteLoading = $state(false);
	let error = $state('');

	// Webhook manager
	let webhookChannelId = $state<string | null>(null);

	// Onboarding
	let showOnboarding = $state(false);
	let onboardingData = $state<{ welcome_message: string | null; prompts: unknown[] } | null>(null);
	let voicePopout = $state(false);

	if (browser) {
		const match = window.location.pathname.match(/\/servers\/([^/]+)/);
		const sid = match?.[1] ?? '';
		spaceId = sid;
		voicePopout = new URLSearchParams(window.location.search).get('voicePopout') === '1';
		if (sid) {
			initializeSpace(sid);
		}
	}

	function initializeSpace(sid: string) {
		fetchChannels(sid).catch(() => {});
		fetchMembers(sid).catch(() => {});
		initUnreadListener();
		initPresenceListener();
		initChannelListeners();
		initRoleListeners();
		initMemberListeners();
		initSpaceListeners();
		loadReadStates().catch(() => {});
		// Check onboarding (show once per server per browser session)
		const seenKey = `onboarding_seen_${sid}`;
		if (!sessionStorage.getItem(seenKey)) {
			api.get<{ enabled: boolean; welcome_message: string | null; prompts: unknown[] }>(`/servers/${sid}/onboarding`)
				.then(d => {
					if (d.enabled) {
						onboardingData = d;
						showOnboarding = true;
					}
				})
				.catch(() => {});
		}
	}

	function dismissOnboarding() {
		showOnboarding = false;
		if (browser && spaceId) {
			sessionStorage.setItem(`onboarding_seen_${spaceId}`, '1');
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
			const sid = spaceId;
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
		if (!spaceId) return;
		inviteLoading = true;
		try {
			const res = await api.post<{ code: string }>(`/servers/${spaceId}/invites`, {});
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

<div class="flex flex-1 {voicePopout ? 'bg-black' : ''}">
	{#if !voicePopout}
	<div use:edgeResize={{ handles: ['right'], minWidth: 224, maxWidth: 384 }} class="w-60 bg-gray-800 flex flex-col flex-shrink-0 overflow-auto" style="min-width: 14rem; max-width: 24rem;">
		<!-- Space header with actions -->
		<div class="h-12 px-3 flex items-center justify-between border-b border-gray-900">
			<h2 class="font-semibold text-white truncate text-sm">
				{$currentSpace?.name ?? 'Space'}
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
				{#if $currentUser?.id === $currentSpace?.owner_id}
					<button
						onclick={() => { window.location.href = `/servers/${spaceId}/settings`; }}
						class="w-6 h-6 rounded flex items-center justify-center text-gray-400 hover:text-white hover:bg-gray-700 text-xs"
						title="Space Settings"
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
							class="px-2 py-1 bg-gray-600 text-white text-xs rounded">Copy</button>
					</div>
				{:else}
					<button onclick={handleCreateInvite} disabled={inviteLoading}
						class="w-full text-xs py-1 bg-gray-600 hover:bg-gray-700 disabled:opacity-50 text-white rounded">
						{inviteLoading ? 'Creating...' : 'Generate Invite Link'}
					</button>
				{/if}
			</div>
		{/if}

		<!-- Channel sidebar -->
		<ChannelSidebar
			spaceId={spaceId}
			{showCreateChannel}
			onCreateChannelToggle={() => showCreateChannel = false}
			onWebhookOpen={(channelId) => webhookChannelId = channelId}
		/>

		<VoicePanel canRecord={$currentUser?.id === $currentSpace?.owner_id} />
		{#if $currentVoiceChannelId && spaceId}
			<SoundboardPanel
				spaceId={spaceId}
				isOwner={$currentUser?.id === $currentSpace?.owner_id}
			/>
		{/if}
		{#if $currentChannel?.channel_type === 3}
			<StagePanel
				channelId={$currentChannelId || ''}
				spaceId={spaceId}
				isOwner={$currentUser?.id === $currentSpace?.owner_id}
			/>
		{/if}
		<UserPanel />
	</div>
	{/if}

	<div class="flex-1 flex flex-col">
		<VoiceStage />
		{#if !voicePopout}
			{@render children()}
		{/if}
	</div>

	{#if !voicePopout}
	<MemberList
		members={$members}
		loading={$membersLoading}
		spaceId={spaceId}
		isOwner={$currentUser?.id === $currentSpace?.owner_id}
		onlineUserIds={new Set($presenceMap.keys())}
	/>
	{/if}
</div>


{#if webhookChannelId}
	<WebhookManager channelId={webhookChannelId} onClose={() => webhookChannelId = null} />
{/if}

{#if showQuickSwitcher}
	<QuickSwitcher onClose={() => showQuickSwitcher = false} />
{/if}

{#if showOnboarding && onboardingData}
	<OnboardingModal
		serverName={$currentSpace?.name ?? 'this server'}
		welcomeMessage={onboardingData.welcome_message}
		prompts={onboardingData.prompts as any[]}
		onDismiss={dismissOnboarding}
	/>
{/if}
