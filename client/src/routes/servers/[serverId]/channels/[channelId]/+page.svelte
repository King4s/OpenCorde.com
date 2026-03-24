<script lang="ts">
	/**
	 * @file Channel message view
	 * @purpose Display messages, send, reply, react, typing indicator
	 * @depends stores/messages, stores/channels, stores/typing
	 * @version 2.1.0
	 */
	import { browser } from '$app/environment';
	import { onMount } from 'svelte';
	import {
		messages,
		fetchMessages,
		sendMessage,
		toggleReaction,
		loading,
		hasMore,
		initMessageListener
	} from '$lib/stores/messages';
	import { currentChannel, selectChannel } from '$lib/stores/channels';
	import { initTypingListener, getTypingForChannel } from '$lib/stores/typing';
	import { ackChannel } from '$lib/stores/unread';
	import MessageList from '$lib/components/chat/MessageList.svelte';
	import MessageInput from '$lib/components/chat/MessageInput.svelte';
	import TypingIndicator from '$lib/components/chat/TypingIndicator.svelte';
	import SearchModal from '$lib/components/chat/SearchModal.svelte';
	import PinsPanel from '$lib/components/chat/PinsPanel.svelte';
	import ThreadPanel from '$lib/components/chat/ThreadPanel.svelte';
	import ChannelSettingsModal from '$lib/components/modals/ChannelSettingsModal.svelte';
	import { threadStore } from '$lib/stores/threads.svelte';
	import api from '$lib/api/client';
	import type { Message, Attachment } from '$lib/api/types';

	let channelId = $state('');
	let serverId = $state('');
	let replyTo = $state<Message | null>(null);
	let showSearch = $state(false);
	let showPins = $state(false);
	let showThread = $state(false);
	let showChannelSettings = $state(false);

	if (browser) {
		const serverMatch = window.location.pathname.match(/\/servers\/([^/]+)/);
		serverId = serverMatch?.[1] ?? '';

		const match = window.location.pathname.match(/\/channels\/([^/]+)/);
		const id = match?.[1] ?? '';
		if (id) {
			channelId = id;
			selectChannel(id);
			fetchMessages(id).then(() => {
				const latest = $messages[$messages.length - 1];
				if (latest) {
					ackChannel(id, latest.id).catch(() => {});
				}
			}).catch(() => {});
		}
	}

	onMount(() => {
		initMessageListener();
		initTypingListener();

		const unsubscribe = messages.subscribe($msgs => {
			if (channelId && $msgs.length > 0) {
				const latest = $msgs[$msgs.length - 1];
				if (latest) {
					ackChannel(channelId, latest.id).catch(() => {});
				}
			}
		});

		return unsubscribe;
	});

	let typingUsers = $derived(channelId ? getTypingForChannel(channelId) : null);

	async function handleSend(content: string, replyToId?: string, attachments?: Attachment[]) {
		if (channelId) {
			await sendMessage(channelId, content, replyToId, attachments);
			replyTo = null;
		}
	}

	async function handleLoadMore() {
		const sorted = [...$messages].sort((a, b) => Number(BigInt(a.id) - BigInt(b.id)));
		const oldest = sorted[0];
		if (oldest && channelId) {
			await fetchMessages(channelId, oldest.id);
		}
	}

	function handleReply(msg: Message) {
		replyTo = msg;
	}

	async function handleReact(msgId: string, emoji: string, currentlyReacted: boolean) {
		await toggleReaction(msgId, emoji, currentlyReacted);
	}

	async function handlePin(msgId: string) {
		try {
			await api.put(`/channels/${channelId}/pins/${msgId}`);
		} catch (e: any) {
			console.error('Failed to pin message', e.message);
		}
	}

	async function handleOpenThread(msgId: string) {
		try {
			await threadStore.createThread(channelId, msgId);
			showThread = true;
		} catch (e: unknown) {
			console.error('Failed to open thread', e);
		}
	}

	function handleChannelSettingsSave(updated: { name: string; topic: string | null; nsfw: boolean }) {
		// Update the current channel in the store with new values
		if ($currentChannel) {
			$currentChannel.name = updated.name;
			$currentChannel.topic = updated.topic;
			$currentChannel.nsfw = updated.nsfw;
		}
	}
</script>

<!-- Channel header -->
<div class="h-12 px-4 flex items-center border-b border-gray-900 shadow-md bg-gray-750 flex-shrink-0">
	<span class="text-gray-400 mr-2 text-sm">#</span>
	<button
		onclick={() => showChannelSettings = true}
		class="font-semibold text-white text-sm hover:text-indigo-400 transition-colors flex items-center gap-1"
		title="Channel settings"
	>
		{$currentChannel?.name ?? ''}
	</button>
	{#if $currentChannel?.topic}
		<span class="text-gray-400 text-xs ml-3 truncate">{$currentChannel.topic}</span>
	{/if}
	<div class="ml-auto flex items-center gap-1">
		<button
			onclick={() => showChannelSettings = true}
			class="text-gray-400 hover:text-white p-1 rounded hover:bg-gray-700/50 transition-colors"
			title="Channel settings"
			aria-label="Channel settings"
		>⚙️</button>
		<button
			onclick={() => showPins = !showPins}
			class="text-gray-400 hover:text-white p-1 rounded hover:bg-gray-700/50 transition-colors {showPins ? 'text-white bg-gray-700/50' : ''}"
			title="Pinned messages"
			aria-label="Pinned messages"
		>📌</button>
		<button
			onclick={() => showSearch = true}
			class="text-gray-400 hover:text-white p-1 rounded hover:bg-gray-700/50 transition-colors"
			title="Search messages"
			aria-label="Search messages"
		>🔍</button>
	</div>
</div>

<!-- Main area: messages + optional pins/thread panels -->
<div class="flex-1 flex overflow-hidden relative">
	<div class="flex-1 flex flex-col overflow-hidden">
		<!-- Messages -->
		<MessageList
			messages={$messages}
			loading={$loading}
			hasMore={$hasMore}
			onLoadMore={handleLoadMore}
			onReply={handleReply}
			onReact={handleReact}
			onPin={handlePin}
			onOpenThread={handleOpenThread}
		/>

		<!-- Typing indicator -->
		{#if typingUsers}
			<TypingIndicator typingUsers={$typingUsers ?? []} />
		{/if}

		<!-- Input -->
		<MessageInput
			onSend={handleSend}
			channelName={$currentChannel?.name ?? ''}
			{channelId}
			{replyTo}
			onCancelReply={() => { replyTo = null; }}
		/>
	</div>
	{#if showPins}
		<PinsPanel {channelId} onClose={() => showPins = false} />
	{/if}
	{#if showThread}
		<ThreadPanel onClose={() => { showThread = false; threadStore.closeThread(); }} />
	{/if}
</div>

{#if showSearch}
	<SearchModal {channelId} {serverId} onClose={() => showSearch = false} />
{/if}

{#if showChannelSettings && $currentChannel}
	<ChannelSettingsModal
		{channelId}
		{serverId}
		channelName={$currentChannel.name}
		channelTopic={$currentChannel.topic ?? null}
		channelNsfw={$currentChannel.nsfw}
		onClose={() => showChannelSettings = false}
		onSave={handleChannelSettingsSave}
	/>
{/if}
