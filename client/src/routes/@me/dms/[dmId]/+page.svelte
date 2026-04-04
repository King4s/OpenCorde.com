<script lang="ts">
	/**
	 * @file DM conversation page
	 * @purpose Shows messages for a DM, message input, markdown rendering
	 * @depends stores/dms, utils/markdown
	 */
	import { browser } from '$app/environment';
	import { page } from '$app/stores';
	import { activeDmMessages, dmLoading, fetchDmMessages, sendDmMessage } from '$lib/stores/dms';
	import { renderMarkdown } from '$lib/utils/markdown';
	import { tick } from 'svelte';

	let { data } = $props();
	let dmId = '';
	let scrollContainer: HTMLDivElement;
	let messageContent = $state('');
	let error = $state('');
	let isSending = $state(false);

	// Load messages on mount
	if (browser) {
		const match = window.location.pathname.match(/\/@me\/dms\/([^/]+)/);
		dmId = match?.[1] ?? '';
		if (dmId) {
			fetchDmMessages(dmId).catch(() => {});
		}
	}

	// Auto-scroll to bottom when messages arrive
	let prevCount = 0;
	$effect(() => {
		const count = $activeDmMessages.length;
		if (count > prevCount && !$dmLoading) {
			tick().then(() => {
				if (scrollContainer) scrollContainer.scrollTop = scrollContainer.scrollHeight;
			});
		}
		prevCount = count;
	});

	/**
	 * Send a message
	 */
	async function handleSend() {
		if (!messageContent.trim() || !dmId || isSending) return;
		error = '';
		isSending = true;
		try {
			await sendDmMessage(dmId, messageContent.trim());
			messageContent = '';
		} catch (e: any) {
			error = e.message || 'Failed to send message';
		} finally {
			isSending = false;
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			handleSend();
		}
	}

	function formatTimestamp(iso: string): string {
		const date = new Date(iso);
		const now = new Date();
		const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
		const yesterday = new Date(today.getTime() - 86_400_000);
		const msgDay = new Date(date.getFullYear(), date.getMonth(), date.getDate());
		const time = date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
		if (msgDay.getTime() === today.getTime()) return `Today at ${time}`;
		if (msgDay.getTime() === yesterday.getTime()) return `Yesterday at ${time}`;
		return `${date.toLocaleDateString()} at ${time}`;
	}

	function getAvatarColor(userId: string): string {
		const colors = [
			'bg-gray-600',
			'bg-gray-600',
			'bg-gray-600',
			'bg-gray-600',
			'bg-gray-600',
			'bg-gray-600',
			'bg-gray-600',
			'bg-gray-600'
		];
		const hash = userId.split('').reduce((acc, c) => acc + c.charCodeAt(0), 0);
		return colors[hash % colors.length];
	}

	function getInitials(name: string): string {
		return name.slice(0, 2).toUpperCase();
	}
</script>

<div class="flex-1 flex flex-col bg-gray-900">
	<!-- Message list -->
	<div bind:this={scrollContainer} class="flex-1 overflow-y-auto px-4 pt-4 pb-2 flex flex-col gap-0">
		{#if $dmLoading && $activeDmMessages.length === 0}
			<div class="flex items-center justify-center h-full">
				<p class="text-gray-500 text-sm">Loading messages...</p>
			</div>
		{:else if $activeDmMessages.length === 0}
			<div class="flex items-center justify-center h-full">
				<p class="text-gray-500 text-sm">No messages yet. Start the conversation!</p>
			</div>
		{:else}
			{#each $activeDmMessages as msg (msg.id)}
				<div class="flex gap-3 py-1 hover:bg-gray-800/40 rounded px-2 group transition-colors relative">
					<!-- Avatar -->
					<div class="flex-shrink-0 w-10 h-10 rounded-full {getAvatarColor(msg.author_id)} flex items-center justify-center text-white text-xs font-semibold mt-0.5">
						{getInitials(msg.author_username)}
					</div>

					<!-- Message content -->
					<div class="flex-1 min-w-0 py-1">
						<div class="flex items-baseline gap-2">
							<span class="text-sm font-medium text-white">{msg.author_username}</span>
							<span class="text-xs text-gray-500 opacity-0 group-hover:opacity-100 transition-opacity">
								{formatTimestamp(msg.created_at)}
							</span>
						</div>
						<div class="text-sm text-gray-300 whitespace-pre-wrap break-words">
							{@html renderMarkdown(msg.content)}
						</div>
					</div>
				</div>
			{/each}
		{/if}
	</div>

	<!-- Message input -->
	<div class="px-4 pb-3 border-t border-gray-800">
		{#if error}
			<p class="text-gray-400 text-xs mb-2">{error}</p>
		{/if}
		<form
			onsubmit={(e) => {
				e.preventDefault();
				handleSend();
			}}
		>
			<div class="flex items-center bg-gray-700 rounded-lg px-4 py-2">
				<input
					type="text"
					bind:value={messageContent}
					onkeydown={handleKeydown}
					placeholder="Send a message..."
					disabled={isSending}
					class="flex-1 py-2 bg-transparent text-white placeholder-gray-400 focus:outline-none text-sm disabled:opacity-50"
				/>
				<button
					type="submit"
					disabled={!messageContent.trim() || isSending}
					class="ml-2 text-gray-400 hover:text-gray-300 disabled:text-gray-600 transition-colors text-sm font-medium"
				>
					{isSending ? 'Sending...' : 'Send'}
				</button>
			</div>
		</form>
	</div>
</div>
