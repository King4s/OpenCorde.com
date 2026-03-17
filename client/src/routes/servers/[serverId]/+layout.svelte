<script lang="ts">
	/**
	 * @file Server layout — channel sidebar + content
	 * @purpose Shows channels for selected server
	 * @depends stores/channels, stores/servers, stores/voice
	 * @version 1.0.0
	 */
	import { browser } from '$app/environment';
	import {
		fetchChannels,
		textChannels,
		voiceChannels,
		selectChannel,
		currentChannelId
	} from '$lib/stores/channels';
	import { currentServer } from '$lib/stores/servers';
	import { joinVoice, inVoice, currentVoiceChannelId } from '$lib/stores/voice';
	import VoicePanel from '$lib/components/voice/VoicePanel.svelte';

	let { children } = $props();
	let serverId = '';

	if (browser) {
		const match = window.location.pathname.match(/\/servers\/([^/]+)/);
		serverId = match?.[1] ?? '';
		if (serverId) {
			fetchChannels(serverId).catch(() => {});
		}
	}

	function handleVoiceJoin(channelId: string) {
		joinVoice(channelId);
	}
</script>

<div class="flex flex-1">
	<!-- Channel sidebar -->
	<div class="w-60 bg-gray-800 flex flex-col">
		<!-- Server name header -->
		<div class="h-12 px-4 flex items-center border-b border-gray-900 shadow-md bg-gray-750">
			<h2 class="font-semibold text-white truncate text-sm">
				{$currentServer?.name ?? 'Server'}
			</h2>
		</div>

		<div class="flex-1 overflow-y-auto p-2 space-y-4">
			<!-- Text channels -->
			{#if $textChannels.length > 0}
				<div>
					<h3 class="text-xs font-semibold text-gray-400 uppercase px-2 mb-1">Text Channels</h3>
					{#each $textChannels as channel (channel.id)}
						<button
							class="w-full px-2 py-1.5 rounded text-left text-sm transition-colors {$currentChannelId ===
							channel.id
								? 'bg-gray-700 text-white'
								: 'text-gray-400 hover:text-gray-200 hover:bg-gray-700/50'}"
							onclick={() => {
								selectChannel(channel.id);
								window.location.href = `/servers/${serverId}/channels/${channel.id}`;
							}}
						>
							<span class="text-gray-500 mr-1">#</span>{channel.name}
						</button>
					{/each}
				</div>
			{/if}

			<!-- Voice channels -->
			{#if $voiceChannels.length > 0}
				<div>
					<h3 class="text-xs font-semibold text-gray-400 uppercase px-2 mb-1">Voice Channels</h3>
					{#each $voiceChannels as channel (channel.id)}
						<button
							class="w-full px-2 py-1.5 rounded text-left text-sm transition-colors {$currentVoiceChannelId ===
							channel.id
								? 'bg-green-700 text-white'
								: 'text-gray-400 hover:text-gray-200 hover:bg-gray-700/50'}"
							onclick={() => handleVoiceJoin(channel.id)}
						>
							<span class="text-gray-500 mr-1">🔊</span>{channel.name}
						</button>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Voice panel at bottom -->
		<VoicePanel />
	</div>

	<!-- Channel content -->
	<div class="flex-1 flex flex-col">
		{@render children()}
	</div>
</div>
