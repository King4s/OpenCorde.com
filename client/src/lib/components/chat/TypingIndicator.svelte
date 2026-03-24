<script lang="ts">
	/**
	 * @file Typing indicator component
	 * @purpose Show animated "X is typing..." indicator below message input
	 */
	interface Props {
		typingUsers: string[];
	}

	let { typingUsers }: Props = $props();

	function formatTyping(users: string[]): string {
		if (users.length === 0) return '';
		if (users.length === 1) return `${users[0]} is typing...`;
		if (users.length === 2) return `${users[0]} and ${users[1]} are typing...`;
		return 'Several people are typing...';
	}
</script>

{#if typingUsers.length > 0}
	<div class="px-4 pb-1 flex items-center gap-1.5 h-5">
		<!-- Animated bouncing dots -->
		<div class="flex gap-0.5 items-end">
			<span
				class="w-1 h-1 bg-gray-400 rounded-full animate-bounce"
				style="animation-delay: 0ms"
			></span>
			<span
				class="w-1 h-1 bg-gray-400 rounded-full animate-bounce"
				style="animation-delay: 150ms"
			></span>
			<span
				class="w-1 h-1 bg-gray-400 rounded-full animate-bounce"
				style="animation-delay: 300ms"
			></span>
		</div>
		<span class="text-xs text-gray-400">{formatTyping(typingUsers)}</span>
	</div>
{:else}
	<!-- Maintain consistent spacing even when no one is typing -->
	<div class="h-5"></div>
{/if}
