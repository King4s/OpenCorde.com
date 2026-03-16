<script lang="ts">
	/**
	 * @file Message input component
	 * @purpose Text input + send button for chat
	 * @version 1.0.0
	 */
	interface Props {
		onSend: (content: string) => void;
		channelName: string;
	}

	let { onSend, channelName }: Props = $props();
	let content = $state('');
	let inputElement: HTMLInputElement;

	function handleSubmit(e: Event) {
		e.preventDefault();
		if (content.trim()) {
			onSend(content.trim());
			content = '';
			inputElement?.focus();
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			handleSubmit(e);
		}
	}
</script>

<form onsubmit={handleSubmit} class="px-4 pb-4">
	<div class="flex items-center bg-gray-700 rounded-lg px-4 py-2">
		<input
			type="text"
			bind:value={content}
			bind:this={inputElement}
			onkeydown={handleKeydown}
			placeholder="Message #{channelName}"
			class="flex-1 py-2 bg-transparent text-white placeholder-gray-400 focus:outline-none text-sm"
		/>
		<button
			type="submit"
			disabled={!content.trim()}
			class="ml-2 text-indigo-400 hover:text-indigo-300 disabled:text-gray-600 transition-colors text-sm font-medium"
		>
			Send
		</button>
	</div>
</form>
