<script lang="ts">
	/**
	 * @file Space layout — channel sidebar + content
	 * @purpose Shows channels, create channel, invite link, channel context menu
	 */
	import { browser } from '$app/environment';
	import { onMount } from 'svelte';
	import { goto, afterNavigate } from '$app/navigation';
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
	let showSidebarDrawer = $state(false);
	let showMembersDrawer = $state(false);
	let isCompactShell = $state(false);

	if (browser) {
		const match = window.location.pathname.match(/\/servers\/([^/]+)/);
		const sid = match?.[1] ?? '';
		spaceId = sid;
		voicePopout = new URLSearchParams(window.location.search).get('voicePopout') === '1';
		isCompactShell = window.matchMedia('(max-width: 1024px)').matches;
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
		const media = window.matchMedia('(max-width: 1024px)');
		const updateCompact = () => {
			isCompactShell = media.matches;
			if (!media.matches) {
				showSidebarDrawer = false;
				showMembersDrawer = false;
			}
		};
		updateCompact();
		media.addEventListener('change', updateCompact);
		afterNavigate(() => {
			showSidebarDrawer = false;
			showMembersDrawer = false;
		});

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
		return () => {
			window.removeEventListener('keydown', handleKeydown);
			media.removeEventListener('change', updateCompact);
		};
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

<div class="flex flex-1 min-h-0 {voicePopout ? 'bg-black' : 'bg-gray-900'}">
	{#if !voicePopout && !isCompactShell}
		<div use:edgeResize={{ handles: ['right'], minWidth: 224, maxWidth: 384 }} class="flex min-h-0 flex-shrink-0 flex-col overflow-auto bg-gray-800" style="width: var(--shell-sidebar-width);">
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

	<div class="flex min-w-0 flex-1 flex-col">
		{#if !voicePopout && isCompactShell}
			<div class="flex items-center gap-2 border-b border-gray-900 bg-gray-800 px-3 py-2 lg:hidden">
				<button
					onclick={() => { showSidebarDrawer = true; showMembersDrawer = false; }}
					class="rounded-lg border border-gray-700 bg-gray-900 px-3 py-1.5 text-xs font-semibold uppercase tracking-[0.14em] text-gray-300"
				>
					Channels
				</button>
				<div class="min-w-0 flex-1">
					<p class="truncate text-xs font-semibold uppercase tracking-[0.16em] text-gray-500">{$currentSpace?.name ?? 'Space'}</p>
					<p class="truncate text-sm text-gray-200">{$currentChannel?.name ?? 'Choose a channel'}</p>
				</div>
				<button
					onclick={() => { showMembersDrawer = true; showSidebarDrawer = false; }}
					class="rounded-lg border border-gray-700 bg-gray-900 px-3 py-1.5 text-xs font-semibold uppercase tracking-[0.14em] text-gray-300"
				>
					Members
				</button>
			</div>
		{/if}
		<VoiceStage />
		{#if !voicePopout}
			<div class="min-h-0 min-w-0 flex-1">
				{@render children()}
			</div>
		{/if}
	</div>

	{#if !voicePopout && !isCompactShell}
		<div class="flex min-h-0 flex-shrink-0" style="width: var(--shell-member-width);">
			<MemberList
				members={$members}
				loading={$membersLoading}
				spaceId={spaceId}
				isOwner={$currentUser?.id === $currentSpace?.owner_id}
				onlineUserIds={new Set($presenceMap.keys())}
			/>
		</div>
	{/if}
</div>

{#if !voicePopout && isCompactShell && showSidebarDrawer}
	<div class="fixed inset-0 z-40 lg:hidden">
		<button type="button" class="absolute inset-0 bg-black/60" aria-label="Close channels drawer" onclick={() => { showSidebarDrawer = false; }}></button>
		<div class="relative z-10 h-full w-[min(90vw,22rem)] overflow-hidden bg-gray-800 shadow-2xl">
			<div class="flex h-full min-h-0 flex-col">
				<div class="flex items-center justify-between border-b border-gray-900 px-3 py-2">
					<p class="text-xs font-semibold uppercase tracking-[0.16em] text-gray-500">Channels</p>
					<button type="button" class="rounded p-1 text-gray-400 hover:bg-gray-700 hover:text-white" onclick={() => { showSidebarDrawer = false; }}>✕</button>
				</div>
				<div class="min-h-0 flex-1 overflow-auto">
					<div class="flex h-full min-h-0 flex-col overflow-auto bg-gray-800" style="width: 100%;">
						<div class="h-12 px-3 flex items-center justify-between border-b border-gray-900">
							<h2 class="font-semibold text-white truncate text-sm">{$currentSpace?.name ?? 'Space'}</h2>
							<div class="flex gap-1">
								<button type="button" onclick={() => { showInvite = !showInvite; showCreateChannel = false; }} class="w-6 h-6 rounded flex items-center justify-center text-gray-400 hover:text-white hover:bg-gray-700 text-xs" title="Create Invite">+&#x1F517;</button>
								<button type="button" onclick={() => { showCreateChannel = !showCreateChannel; showInvite = false; }} class="w-6 h-6 rounded flex items-center justify-center text-gray-400 hover:text-white hover:bg-gray-700 text-xs" title="Create Channel">+#</button>
							</div>
						</div>
						{#if showInvite}
							<div class="p-2 bg-gray-750 border-b border-gray-900">
								{#if inviteCode}
									<div class="flex gap-1">
										<input type="text" value={inviteCode} readonly class="flex-1 min-w-0 px-2 py-1 bg-gray-900 border border-gray-700 rounded text-white text-xs" />
										<button type="button" onclick={copyInvite} class="px-2 py-1 bg-gray-600 text-white text-xs rounded">Copy</button>
									</div>
								{:else}
									<button type="button" onclick={handleCreateInvite} disabled={inviteLoading} class="w-full text-xs py-1 bg-gray-600 hover:bg-gray-700 disabled:opacity-50 text-white rounded">{inviteLoading ? 'Creating...' : 'Generate Invite Link'}</button>
								{/if}
							</div>
						{/if}
						<ChannelSidebar spaceId={spaceId} {showCreateChannel} onCreateChannelToggle={() => showCreateChannel = false} onWebhookOpen={(channelId) => webhookChannelId = channelId} />
						<VoicePanel canRecord={$currentUser?.id === $currentSpace?.owner_id} />
						{#if $currentVoiceChannelId && spaceId}
							<SoundboardPanel spaceId={spaceId} isOwner={$currentUser?.id === $currentSpace?.owner_id} />
						{/if}
						{#if $currentChannel?.channel_type === 3}
							<StagePanel channelId={$currentChannelId || ''} spaceId={spaceId} isOwner={$currentUser?.id === $currentSpace?.owner_id} />
						{/if}
						<UserPanel />
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}

{#if !voicePopout && isCompactShell && showMembersDrawer}
	<div class="fixed inset-0 z-40 lg:hidden">
		<button type="button" class="absolute inset-0 bg-black/60" aria-label="Close members drawer" onclick={() => { showMembersDrawer = false; }}></button>
		<div class="relative z-10 ml-auto h-full w-[min(88vw,20rem)] overflow-hidden bg-gray-800 shadow-2xl">
			<div class="flex h-full min-h-0 flex-col">
				<div class="flex items-center justify-between border-b border-gray-900 px-3 py-2">
					<p class="text-xs font-semibold uppercase tracking-[0.16em] text-gray-500">Members</p>
					<button type="button" class="rounded p-1 text-gray-400 hover:bg-gray-700 hover:text-white" onclick={() => { showMembersDrawer = false; }}>✕</button>
				</div>
				<div class="min-h-0 flex-1 overflow-auto">
					<MemberList members={$members} loading={$membersLoading} spaceId={spaceId} isOwner={$currentUser?.id === $currentSpace?.owner_id} onlineUserIds={new Set($presenceMap.keys())} />
				</div>
			</div>
		</div>
	</div>
{/if}


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
