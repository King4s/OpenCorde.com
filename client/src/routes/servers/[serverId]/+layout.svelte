<script lang="ts">
	/**
	 * @file Server layout — channel sidebar + content
	 * @purpose Shows channels, create channel, invite link
	 */
	import { browser } from '$app/environment';
	import {
		fetchChannels,
		textChannels,
		voiceChannels,
		selectChannel,
		currentChannelId,
		createChannel
	} from '$lib/stores/channels';
	import { currentServer } from '$lib/stores/servers';
	import { joinVoice, currentVoiceChannelId } from '$lib/stores/voice';
	import api from '$lib/api/client';
	import VoicePanel from '$lib/components/voice/VoicePanel.svelte';
	import MemberList from '$lib/components/layout/MemberList.svelte';
	import { members, membersLoading, fetchMembers } from '$lib/stores/members';

	let { children } = $props();
	let serverId = '';

	// Modals
	let showCreateChannel = $state(false);
	let showInvite = $state(false);
	let channelName = $state('');
	let channelType = $state(0);
	let inviteCode = $state('');
	let inviteLoading = $state(false);
	let error = $state('');

	if (browser) {
		const match = window.location.pathname.match(/\/servers\/([^/]+)/);
		serverId = match?.[1] ?? '';
		if (serverId) {
			fetchChannels(serverId).catch(() => {});
			fetchMembers(serverId).catch(() => {});
		}
	}

	async function handleCreateChannel() {
		if (!channelName.trim() || !serverId) return;
		error = '';
		try {
			await createChannel(serverId, channelName.trim(), channelType);
			showCreateChannel = false;
			channelName = '';
			channelType = 0;
		} catch (e: any) {
			error = e.message || 'Failed to create channel';
		}
	}

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
			</div>
		</div>

		<!-- Create channel form -->
		{#if showCreateChannel}
			<div class="p-2 bg-gray-750 border-b border-gray-900">
				{#if error}<p class="text-red-400 text-xs mb-1">{error}</p>{/if}
				<input type="text" bind:value={channelName} placeholder="channel-name"
					class="w-full px-2 py-1 bg-gray-900 border border-gray-700 rounded text-white text-xs placeholder-gray-500 focus:outline-none focus:border-indigo-500 mb-1" />
				<div class="flex gap-1 mb-1">
					<button onclick={() => channelType = 0}
						class="flex-1 text-xs py-1 rounded {channelType === 0 ? 'bg-indigo-600 text-white' : 'bg-gray-700 text-gray-400'}">Text</button>
					<button onclick={() => channelType = 1}
						class="flex-1 text-xs py-1 rounded {channelType === 1 ? 'bg-indigo-600 text-white' : 'bg-gray-700 text-gray-400'}">Voice</button>
				</div>
				<button onclick={handleCreateChannel} disabled={!channelName.trim()}
					class="w-full text-xs py-1 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white rounded">Create</button>
			</div>
		{/if}

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

		<div class="flex-1 overflow-y-auto p-2 space-y-4">
			{#if $textChannels.length > 0}
				<div>
					<h3 class="text-xs font-semibold text-gray-400 uppercase px-2 mb-1">Text Channels</h3>
					{#each $textChannels as channel (channel.id)}
						<button
							class="w-full px-2 py-1.5 rounded text-left text-sm transition-colors {$currentChannelId === channel.id ? 'bg-gray-700 text-white' : 'text-gray-400 hover:text-gray-200 hover:bg-gray-700/50'}"
							onclick={() => { selectChannel(channel.id); window.location.href = `/servers/${serverId}/channels/${channel.id}`; }}
						>
							<span class="text-gray-500 mr-1">#</span>{channel.name}
						</button>
					{/each}
				</div>
			{/if}

			{#if $voiceChannels.length > 0}
				<div>
					<h3 class="text-xs font-semibold text-gray-400 uppercase px-2 mb-1">Voice Channels</h3>
					{#each $voiceChannels as channel (channel.id)}
						<button
							class="w-full px-2 py-1.5 rounded text-left text-sm transition-colors {$currentVoiceChannelId === channel.id ? 'bg-green-700 text-white' : 'text-gray-400 hover:text-gray-200 hover:bg-gray-700/50'}"
							onclick={() => joinVoice(channel.id)}
						>
							<span class="text-gray-500 mr-1">&#x1F50A;</span>{channel.name}
						</button>
					{/each}
				</div>
			{/if}
		</div>

		<VoicePanel />
	</div>

	<div class="flex-1 flex flex-col">
		{@render children()}
	</div>

	<MemberList members={$members} loading={$membersLoading} />
</div>
