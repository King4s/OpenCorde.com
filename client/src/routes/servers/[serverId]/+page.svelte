<script lang="ts">
	/**
	 * @file Space home — shown when space is selected but no channel is active
	 * @purpose Space guide: overview of the space with channels, member count, welcome
	 * @version 2.0.0 — Space Guide
	 */
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import { currentSpace } from '$lib/stores/servers';
	import { channels } from '$lib/stores/channels';
	import { members } from '$lib/stores/members';
	import { goto } from '$app/navigation';
	import api from '$lib/api/client';
	import type { Channel } from '$lib/api/types';

	interface OnboardingData {
		enabled: boolean;
		welcome_message: string | null;
		prompts: unknown[];
	}

	let serverId = $state('');
	let onboarding = $state<OnboardingData | null>(null);
	let loadingOnboarding = $state(false);

	if (browser) {
		const match = window.location.pathname.match(/\/servers\/([^/]+)/);
		serverId = match?.[1] ?? '';
	}

	onMount(() => {
		if (!serverId) return;
		loadingOnboarding = true;
		api.get<OnboardingData>(`/servers/${serverId}/onboarding`)
			.then(d => { onboarding = d; })
			.catch(() => { onboarding = null; })
			.finally(() => { loadingOnboarding = false; });
	});

	const rootText = $derived(
		$channels.filter(c => (c.channel_type === 0 || c.channel_type === 4) && !c.parent_id).sort((a, b) => a.position - b.position)
	);

	const categories = $derived(
		$channels.filter(c => c.channel_type === 2).sort((a, b) => a.position - b.position)
	);

	const forumChannels = $derived(
		$channels.filter(c => c.channel_type === 5 && !c.parent_id).sort((a, b) => a.position - b.position)
	);

	function childrenOf(catId: string) {
		return $channels
			.filter(c => c.parent_id === catId)
			.sort((a, b) => a.position - b.position);
	}

	function channelIcon(channel: Channel) {
		if (channel.channel_type === 1) return '🔊';
		if (channel.channel_type === 3) return '🎙️';
		if (channel.channel_type === 4) return '📢';
		if (channel.channel_type === 5) return '🗂️';
		if (channel.channel_type === 2) return '📁';
		return '#';
	}

	function navigateToChannel(channel: Channel) {
		if (channel.channel_type === 2) return; // category, skip
		const base = channel.channel_type === 5 ? 'forum' : 'channels';
		goto(`/servers/${serverId}/${base}/${channel.id}`);
	}

	const textChannelCount = $derived($channels.filter(c => c.channel_type === 0 || c.channel_type === 4).length);
	const forumChannelCount = $derived(forumChannels.length);
	const voiceChannelCount = $derived($channels.filter(c => c.channel_type === 1 || c.channel_type === 3).length);
</script>

<div class="flex-1 overflow-y-auto bg-gray-900">
	<!-- Space banner / header -->
	<div class="bg-gradient-to-b from-gray-900 to-gray-900 px-4 py-6 sm:px-8 sm:py-10">
		<div class="flex items-center gap-3 sm:gap-4 mb-4">
			{#if $currentSpace?.icon_url}
				<img src={$currentSpace.icon_url} alt="Space icon"
					class="w-16 h-16 rounded-2xl shadow-lg" />
			{:else}
				<div class="w-16 h-16 rounded-2xl bg-gray-700 flex items-center justify-center text-2xl font-bold text-white shadow-lg">
					{($currentSpace?.name ?? '?')[0].toUpperCase()}
				</div>
			{/if}
			<div>
				<h1 class="text-2xl font-bold text-white">{$currentSpace?.name ?? 'Space'}</h1>
				{#if $currentSpace?.description}
					<p class="text-gray-300 mt-1 text-sm">{$currentSpace.description}</p>
				{/if}
			</div>
		</div>

		<!-- Stats row -->
		<div class="flex flex-wrap gap-x-6 gap-y-2 text-sm text-gray-400">
			<span>
				<span class="text-white font-semibold">{$members.length}</span>
				{$members.length === 1 ? 'member' : 'members'}
			</span>
			<span>
				<span class="text-white font-semibold">{textChannelCount}</span>
				text {textChannelCount === 1 ? 'channel' : 'channels'}
			</span>
			{#if forumChannelCount > 0}
				<span>
					<span class="text-white font-semibold">{forumChannelCount}</span>
					forum {forumChannelCount === 1 ? 'channel' : 'channels'}
				</span>
			{/if}
			{#if voiceChannelCount > 0}
				<span>
					<span class="text-white font-semibold">{voiceChannelCount}</span>
					voice {voiceChannelCount === 1 ? 'channel' : 'channels'}
				</span>
			{/if}
		</div>
	</div>

	<div class="px-4 py-5 sm:px-8 sm:py-6 max-w-3xl">
		<!-- Welcome / onboarding message -->
		{#if onboarding?.enabled && onboarding?.welcome_message}
			<div class="bg-gray-900/40 border border-gray-700/50 rounded-xl p-5 mb-6">
				<h2 class="text-base font-semibold text-gray-300 mb-2">Welcome!</h2>
				<p class="text-gray-200 text-sm leading-relaxed">{onboarding.welcome_message}</p>
			</div>
		{/if}

		<!-- Channel overview -->
		<h2 class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-3">Channels</h2>

		{#if rootText.length > 0}
			<div class="mb-4">
				<h3 class="text-xs font-semibold text-gray-500 uppercase tracking-wider px-1 mb-1">Text Channels</h3>
				<div class="grid grid-cols-1 sm:grid-cols-2 gap-1">
					{#each rootText as ch (ch.id)}
						<button
							class="flex items-center gap-2 px-3 py-2 rounded-lg bg-gray-800 hover:bg-gray-700 text-left transition-colors group"
							onclick={() => navigateToChannel(ch)}
						>
							<span class="text-gray-500 group-hover:text-gray-300 text-sm">{channelIcon(ch)}</span>
							<span class="text-gray-300 group-hover:text-white text-sm truncate">{ch.name}</span>
							{#if ch.nsfw}
								<span class="ml-auto text-xs px-1 py-0.5 bg-gray-900/60 text-gray-400 rounded">18+</span>
							{/if}
						</button>
					{/each}
				</div>
			</div>
		{/if}

		{#if forumChannels.length > 0}
			<div class="mb-4">
				<h3 class="text-xs font-semibold text-gray-500 uppercase tracking-wider px-1 mb-1">Forum Channels</h3>
				<div class="grid grid-cols-1 sm:grid-cols-2 gap-1">
					{#each forumChannels as ch (ch.id)}
						<button
							class="flex items-center gap-2 px-3 py-2 rounded-lg bg-gray-800 hover:bg-gray-700 text-left transition-colors group"
							onclick={() => navigateToChannel(ch)}
						>
							<span class="text-gray-500 group-hover:text-gray-300 text-sm">{channelIcon(ch)}</span>
							<span class="text-gray-300 group-hover:text-white text-sm truncate">{ch.name}</span>
							{#if ch.nsfw}
								<span class="ml-auto text-xs px-1 py-0.5 bg-gray-900/60 text-gray-400 rounded">18+</span>
							{/if}
						</button>
					{/each}
				</div>
			</div>
		{/if}

		{#if categories.length > 0}
			<div class="space-y-4">
				{#each categories as cat (cat.id)}
					{@const children = childrenOf(cat.id)}
					{#if children.length > 0}
						<div>
							<h3 class="text-xs font-semibold text-gray-500 uppercase tracking-wider px-1 mb-1">{cat.name}</h3>
							<div class="grid grid-cols-1 sm:grid-cols-2 gap-1">
								{#each children as ch (ch.id)}
									<button
										class="flex items-center gap-2 px-3 py-2 rounded-lg bg-gray-800 hover:bg-gray-700 text-left transition-colors group"
										onclick={() => navigateToChannel(ch)}
									>
										<span class="text-gray-500 group-hover:text-gray-300 text-sm">{channelIcon(ch)}</span>
										<span class="text-gray-300 group-hover:text-white text-sm truncate">{ch.name}</span>
										{#if ch.nsfw}
											<span class="ml-auto text-xs px-1 py-0.5 bg-gray-900/60 text-gray-400 rounded">18+</span>
										{/if}
									</button>
								{/each}
							</div>
						</div>
					{/if}
				{/each}
			</div>
		{/if}

		{#if rootText.length === 0 && forumChannels.length === 0 && categories.length === 0}
			<p class="text-gray-500 text-sm text-center py-8">No channels yet. Create one to get started.</p>
		{/if}
	</div>
</div>
