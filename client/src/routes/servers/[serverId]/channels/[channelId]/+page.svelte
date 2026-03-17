<script lang="ts">
	/**
	 * @file Channel message view
	 * @purpose Display messages, send new messages
	 * @depends stores/messages, stores/channels
	 * @version 1.0.0
	 */
	import { browser } from '$app/environment';
	import { onMount } from 'svelte';
	import {
		messages,
		fetchMessages,
		sendMessage,
		loading,
		hasMore,
		initMessageListener
	} from '$lib/stores/messages';
	import { currentChannel, selectChannel } from '$lib/stores/channels';
	import MessageList from '$lib/components/chat/MessageList.svelte';
	import MessageInput from '$lib/components/chat/MessageInput.svelte';

	let channelId = '';

	if (browser) {
		const match = window.location.pathname.match(/\/channels\/([^/]+)/);
		channelId = match?.[1] ?? '';
		if (channelId) {
			selectChannel(channelId);
			fetchMessages(channelId).catch(() => {});
		}
	}

	onMount(() => {
		initMessageListener();
	});

	async function handleSend(content: string) {
		if (channelId) {
			await sendMessage(channelId, content);
		}
	}

	async function handleLoadMore() {
		const oldest = $messages[0];
		if (oldest && channelId) {
			await fetchMessages(channelId, oldest.id);
		}
	}
</script>

<!-- Channel header -->
<div class="h-12 px-4 flex items-center border-b border-gray-900 shadow-md bg-gray-750">
	<span class="text-gray-400 mr-2 text-sm">#</span>
	<h3 class="font-semibold text-white text-sm">{$currentChannel?.name ?? ''}</h3>
	{#if $currentChannel?.topic}
		<span class="text-gray-400 text-xs ml-3 truncate">{$currentChannel.topic}</span>
	{/if}
</div>

<!-- Messages -->
<MessageList
	messages={$messages}
	loading={$loading}
	hasMore={$hasMore}
	onLoadMore={handleLoadMore}
/>

<!-- Input -->
<MessageInput onSend={handleSend} channelName={$currentChannel?.name ?? ''} />
