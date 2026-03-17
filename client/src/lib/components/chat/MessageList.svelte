<script lang="ts">
	/**
	 * @file Message list component
	 * @purpose Displays messages with author, timestamp, content
	 * @version 1.0.0
	 */
	import type { Message } from '$lib/api/types';

	interface Props {
		messages: Message[];
		loading: boolean;
		hasMore: boolean;
		onLoadMore: () => void;
	}

	let { messages, loading, hasMore, onLoadMore }: Props = $props();

	function formatTime(iso: string): string {
		return new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	}

	function getInitials(name: string): string {
		return name.slice(0, 2).toUpperCase();
	}

	function getAvatarColor(authorId: string): string {
		const colors = [
			'bg-indigo-600',
			'bg-purple-600',
			'bg-pink-600',
			'bg-red-600',
			'bg-orange-600',
			'bg-yellow-600',
			'bg-green-600',
			'bg-teal-600'
		];
		const hash = authorId
			.split('')
			.reduce((acc, char) => acc + char.charCodeAt(0), 0);
		return colors[hash % colors.length];
	}
</script>

<div class="flex-1 overflow-y-auto p-4 space-y-1 flex flex-col justify-end">
	{#if hasMore && !loading}
		<button
			class="text-indigo-400 text-sm hover:underline py-2 self-center transition-colors"
			onclick={onLoadMore}
		>
			Load older messages
		</button>
	{:else if hasMore && loading}
		<div class="text-gray-500 text-sm py-2 self-center">Loading...</div>
	{/if}

	<!-- Messages in reverse order (oldest first, newest at bottom) -->
	{#each [...messages].reverse() as msg (msg.id)}
		<div class="flex gap-3 py-1 hover:bg-gray-800/50 rounded px-2 group transition-colors">
			<!-- Avatar placeholder -->
			<div
				class="w-10 h-10 rounded-full {getAvatarColor(msg.author_id)} flex items-center justify-center text-white text-xs font-semibold flex-shrink-0 mt-0.5"
			>
				{getInitials(msg.author_username)}
			</div>

			<div class="flex-1 min-w-0">
				<div class="flex items-baseline gap-2">
					<span class="font-medium text-white text-sm">{msg.author_username}</span>
					<span class="text-xs text-gray-500">{formatTime(msg.created_at)}</span>
					{#if msg.edited_at}
						<span class="text-xs text-gray-600">(edited)</span>
					{/if}
				</div>
				<p class="text-gray-300 text-sm break-words whitespace-pre-wrap">{msg.content}</p>
			</div>
		</div>
	{/each}
</div>
