<script lang="ts">
	/**
	 * @file Message context menu buttons
	 * @purpose Hover action buttons: reply, thread, pin, edit, delete, copy text, copy link
	 * @depends api/types
	 */
	import type { Message } from '$lib/api/types';

	interface Props {
		msg: Message;
		currentUserId?: string;
		serverId?: string;
		onReply?: (msg: Message) => void;
		onPin?: (msgId: string) => void;
		onOpenThread?: (msgId: string) => void;
		onStartEdit?: (msg: Message) => void;
		onDelete?: (msgId: string) => void;
	}

	let { msg, currentUserId, serverId, onReply, onPin, onOpenThread, onStartEdit, onDelete }: Props = $props();

	const isOwn = $derived(currentUserId === msg.author_id);

	let copied = $state(false);

	function copyText() {
		navigator.clipboard.writeText(msg.content).then(() => {
			copied = true;
			setTimeout(() => { copied = false; }, 1500);
		});
	}

	function copyLink() {
		const path = serverId
			? `/servers/${serverId}/channels/${msg.channel_id}?msg=${msg.id}`
			: `/@me/dms/${msg.channel_id}?msg=${msg.id}`;
		navigator.clipboard.writeText(window.location.origin + path);
	}
</script>

<div class="absolute right-2 top-1 flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
	{#if onOpenThread}
		<button class="text-gray-500 hover:text-indigo-400 text-xs px-1.5 py-0.5 rounded bg-gray-800/80 transition-colors"
			onclick={() => onOpenThread?.(msg.id)} title="Open thread" aria-label="Open thread">🧵</button>
	{/if}
	{#if onPin}
		<button class="text-gray-500 hover:text-yellow-400 text-xs px-1.5 py-0.5 rounded bg-gray-800/80 transition-colors"
			onclick={() => onPin?.(msg.id)} title="Pin message" aria-label="Pin message">📌</button>
	{/if}
	{#if onReply}
		<button class="text-gray-500 hover:text-gray-300 text-xs px-1.5 py-0.5 rounded bg-gray-800/80"
			onclick={() => onReply?.(msg)} title="Reply" aria-label="Reply">↩</button>
	{/if}
	{#if isOwn && onStartEdit}
		<button class="text-gray-500 hover:text-blue-400 text-xs px-1.5 py-0.5 rounded bg-gray-800/80 transition-colors"
			onclick={() => onStartEdit?.(msg)} title="Edit message" aria-label="Edit message">✏</button>
	{/if}
	{#if isOwn && onDelete}
		<button class="text-gray-500 hover:text-red-400 text-xs px-1.5 py-0.5 rounded bg-gray-800/80 transition-colors"
			onclick={() => onDelete?.(msg.id)} title="Delete message" aria-label="Delete message">🗑</button>
	{/if}
	<button class="text-gray-500 hover:text-gray-300 text-xs px-1.5 py-0.5 rounded bg-gray-800/80 transition-colors"
		onclick={copyText} title="Copy text" aria-label="Copy message text">{copied ? '✓' : '⎘'}</button>
	<button class="text-gray-500 hover:text-gray-300 text-xs px-1.5 py-0.5 rounded bg-gray-800/80 transition-colors"
		onclick={copyLink} title="Copy message link" aria-label="Copy link to message">🔗</button>
</div>
