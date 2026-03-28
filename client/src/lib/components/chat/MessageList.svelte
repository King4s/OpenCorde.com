<script lang="ts">
	/**
	 * @file Message list component
	 * @purpose Displays messages with markdown, grouping, timestamps, reply context, reactions
	 * @version 2.3.0 — adds compact message style support
	 */
	import { tick } from 'svelte';
	import { get } from 'svelte/store';
	import type { Message } from '$lib/api/types';
	import { themeStore } from '$lib/stores/theme';
	import MarkdownContent from './MarkdownContent.svelte';
	import EmojiPicker from './EmojiPicker.svelte';
	import MessageContextMenu from './MessageContextMenu.svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { currentChannelId } from '$lib/stores/channels';
	import { getGroupState } from '$lib/stores/e2ee';
	import UserProfilePopover from '$lib/components/user/UserProfilePopover.svelte';

	const messageStyle = themeStore.messageStyle;

	interface Props {
		messages: Message[];
		loading: boolean;
		hasMore: boolean;
		currentUserId?: string;
		serverId?: string;
		onLoadMore: () => void;
		onReply?: (msg: Message) => void;
		onReact?: (msgId: string, emoji: string, currentlyReacted: boolean) => void;
		onPin?: (msgId: string) => void;
		onOpenThread?: (msgId: string) => void;
		onEdit?: (msgId: string, content: string) => void;
		onDelete?: (msgId: string) => void;
		startEditMsgId?: string | null;
		/** ID of the last-read message for this channel — shows NEW divider above first unread */
		lastReadId?: string | null;
	}

	let { messages, loading, hasMore, currentUserId, serverId, onLoadMore, onReply, onReact, onPin, onOpenThread, onEdit, onDelete, startEditMsgId = null, lastReadId = null }: Props = $props();

	let editingMsgId = $state<string | null>(null);
	let editContent = $state('');
	let popoverUserId = $state<string | null>(null);
	let popoverAnchorRect = $state<DOMRect | null>(null);

	function openProfilePopover(userId: string, e: MouseEvent) {
		const el = e.currentTarget as HTMLElement;
		popoverUserId = userId;
		popoverAnchorRect = el.getBoundingClientRect();
	}
	function startEdit(msg: Message) { editingMsgId = msg.id; editContent = msg.content; }

	// The first message after the divider (first msg with ID > lastReadId that isn't the latest msg)
	const dividerBeforeId = $derived.by(() => {
		if (!lastReadId || !messages.length) return null;
		// Don't show divider if we're fully caught up (last msg IS last read)
		const latestId = messages[messages.length - 1]?.id;
		if (latestId === lastReadId) return null;
		const first = messages.find(m => BigInt(m.id) > BigInt(lastReadId));
		return first?.id ?? null;
	});

	function scrollToMessage(msgId: string) {
		const el = document.getElementById('msg-' + msgId);
		if (el) {
			el.scrollIntoView({ behavior: 'smooth', block: 'center' });
			el.classList.add('highlight-flash');
			setTimeout(() => el.classList.remove('highlight-flash'), 1500);
		}
	}
	function cancelEdit() { editingMsgId = null; }
	async function submitEdit() { if (editingMsgId) { await onEdit?.(editingMsgId, editContent); editingMsgId = null; } }
	let scrollContainer: HTMLDivElement;

	// Start editing when parent requests it
	$effect(() => {
		if (startEditMsgId) {
			const msg = messages.find(m => m.id === startEditMsgId);
			if (msg) startEdit(msg);
		}
	});

	// Auto-scroll to bottom when new messages arrive
	let prevCount = 0;
	$effect(() => {
		const count = messages.length;
		if (count > prevCount && !loading) {
			tick().then(() => {
				if (scrollContainer) scrollContainer.scrollTop = scrollContainer.scrollHeight;
			});
		}
		prevCount = count;
	});

	/** Relative timestamp: "Today at 3:00 PM", "Yesterday at 5:30 PM", or "03/15/2026" */
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

	function formatShortTime(iso: string): string {
		return new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	}

	function getInitials(name: string): string {
		return name.slice(0, 2).toUpperCase();
	}

	function getAvatarColor(authorId: string): string {
		const colors = [
			'bg-indigo-600', 'bg-purple-600', 'bg-pink-600', 'bg-red-600',
			'bg-orange-600', 'bg-yellow-600', 'bg-green-600', 'bg-teal-600'
		];
		const hash = authorId.split('').reduce((acc, c) => acc + c.charCodeAt(0), 0);
		return colors[hash % colors.length];
	}

	/** A message is grouped (compact) if same author within 5 minutes of the previous message */
	function isGrouped(prev: Message, curr: Message): boolean {
		if (prev.author_id !== curr.author_id) return false;
		const diff = new Date(curr.created_at).getTime() - new Date(prev.created_at).getTime();
		return diff < 5 * 60 * 1000;
	}

	// Quick emoji picker options and full picker state
	const quickEmojis = ['👍', '❤️', '😂', '😮', '😢', '🎉', '🚀', '👀'];
	let emojiPickerMsgId = $state<string | null>(null);
	let fullPickerMsgId = $state<string | null>(null);

	function toggleEmojiPicker(msgId: string) {
		emojiPickerMsgId = emojiPickerMsgId === msgId ? null : msgId;
	}

	function openFullPicker(msgId: string) {
		emojiPickerMsgId = null;
		fullPickerMsgId = msgId;
	}

	function handleFullEmojiSelect(emoji: string) {
		if (fullPickerMsgId) {
			onReact?.(fullPickerMsgId, emoji, false);
		}
		fullPickerMsgId = null;
	}

	function formatFileSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	/** Detect encrypted attachments: filename ends with .enc */
	function isEncrypted(filename: string): boolean {
		return filename.endsWith('.enc');
	}

	/** Original filename without .enc suffix */
	function originalName(filename: string): string {
		return filename.endsWith('.enc') ? filename.slice(0, -4) : filename;
	}

	/** Fetch encrypted blob, decrypt via Tauri, trigger browser download. */
	async function decryptAndDownload(url: string, filename: string): Promise<void> {
		const channelId = get(currentChannelId);
		if (!channelId) return;
		const groupState = getGroupState(channelId);
		if (!groupState) {
			alert('No E2EE group state for this channel. Re-open the channel or re-join the group.');
			return;
		}
		try {
			const res = await fetch(url);
			const buf = await res.arrayBuffer();
			// ArrayBuffer → base64
			const bytes = new Uint8Array(buf);
			let binary = '';
			for (const b of bytes) binary += String.fromCharCode(b);
			const encryptedB64 = btoa(binary);

			const decryptedB64 = await invoke<string>('crypto_decrypt_file', {
				group_state_hex: groupState,
				encrypted_b64: encryptedB64
			});

			// base64 → Blob → download
			const decBinary = atob(decryptedB64);
			const decBytes = new Uint8Array(decBinary.length);
			for (let i = 0; i < decBinary.length; i++) decBytes[i] = decBinary.charCodeAt(i);
			const blob = new Blob([decBytes]);
			const blobUrl = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = blobUrl;
			a.download = originalName(filename);
			a.click();
			URL.revokeObjectURL(blobUrl);
		} catch (err: any) {
			alert(`Decryption failed: ${err?.message ?? err}`);
		}
	}

	// Compute sorted+grouped array reactively
	let displayMessages = $derived(
		[...messages]
			.sort((a, b) => Number(BigInt(a.id) - BigInt(b.id)))
			.map((msg, i, arr) => ({
				msg,
				compact: i > 0 && isGrouped(arr[i - 1], msg)
			}))
	);
</script>

<div bind:this={scrollContainer} class="flex-1 overflow-y-auto px-4 pt-4 pb-2 flex flex-col gap-0">
	{#if hasMore && !loading}
		<button class="text-indigo-400 text-sm hover:underline py-2 self-center" onclick={onLoadMore}>
			Load older messages
		</button>
	{:else if hasMore && loading}
		<div class="text-gray-500 text-sm py-2 self-center">Loading...</div>
	{/if}

	{#each displayMessages as { msg, compact } (msg.id)}
		{#if dividerBeforeId === msg.id}
			<div class="flex items-center gap-3 px-4 py-1 select-none" aria-label="New messages divider">
				<div class="flex-1 h-px bg-red-500/60"></div>
				<span class="text-red-400 text-xs font-semibold whitespace-nowrap px-1">NEW MESSAGES</span>
				<div class="flex-1 h-px bg-red-500/60"></div>
			</div>
		{/if}
		<div
			id="msg-{msg.id}"
			class="flex gap-3 {$messageStyle === 'compact' ? 'py-0.5 px-1' : compact ? 'py-0.5 px-2' : 'pt-3 pb-0.5 px-2'} hover:bg-gray-800/40 rounded group transition-colors relative"
		>
			{#if $messageStyle === 'compact'}
				<!-- Compact style: minimal avatar or hidden, inline timestamp -->
				<div class="w-4 flex-shrink-0 flex items-center justify-center text-xs text-gray-600 select-none">
					{formatShortTime(msg.created_at).split(':')[0]}
				</div>
			{:else if compact}
				<!-- Cozy grouped: no avatar, show time on hover -->
				<div class="w-10 flex-shrink-0 flex items-center justify-end">
					<span class="text-xs text-gray-600 opacity-0 group-hover:opacity-100 transition-opacity pr-1 select-none">
						{formatShortTime(msg.created_at)}
					</span>
				</div>
			{:else}
				<!-- Cozy full: avatar + username + timestamp -->
				<button
					class="w-10 h-10 rounded-full {getAvatarColor(msg.author_id)} flex items-center justify-center text-white text-xs font-semibold flex-shrink-0 mt-0.5 hover:opacity-90 transition-opacity"
					onclick={(e) => openProfilePopover(msg.author_id, e)}
					title="View profile"
					aria-label="View {msg.author_username}'s profile"
				>
					{getInitials(msg.author_username)}
				</button>
			{/if}

			<div class="flex-1 min-w-0">
				{#if $messageStyle === 'compact'}
					<!-- Compact: username and time inline -->
					<div class="flex items-baseline gap-2 text-xs mb-0.5">
						<button class="font-medium text-white hover:underline" onclick={(e) => openProfilePopover(msg.author_id, e)}>{msg.author_username}</button>
						<span class="text-gray-500">{formatShortTime(msg.created_at)}</span>
						{#if msg.edited_at}
							<span class="text-gray-600">(edited)</span>
						{/if}
					</div>
				{:else if !compact}
					<!-- Cozy: username and timestamp on separate line -->
					<div class="flex items-baseline gap-2 mb-0.5">
						<button class="font-medium text-white text-sm hover:underline" onclick={(e) => openProfilePopover(msg.author_id, e)}>{msg.author_username}</button>
						<span class="text-xs text-gray-500">{formatTimestamp(msg.created_at)}</span>
						{#if msg.edited_at}
							<span class="text-xs text-gray-600">(edited)</span>
						{/if}
					</div>
				{/if}

				<!-- Reply context -->
				{#if msg.reply_to}
					<div class="flex items-center gap-1 mb-1 text-xs text-gray-500 cursor-pointer hover:text-gray-300"
						role="button" tabindex="0"
						onclick={() => msg.reply_to_id && scrollToMessage(msg.reply_to_id)}
						onkeydown={(e) => e.key === 'Enter' && msg.reply_to_id && scrollToMessage(msg.reply_to_id)}>
						<span class="text-gray-600">↩</span>
						<span class="font-medium text-indigo-400">{msg.reply_to.author_username}</span>
						<span class="truncate max-w-xs">{msg.reply_to.content}</span>
					</div>
				{/if}

				<!-- Message content rendered as markdown with syntax highlighting -->
				{#if editingMsgId === msg.id}
					<textarea class="w-full bg-gray-700 text-gray-200 text-sm rounded p-2 resize-none border border-gray-600 focus:border-indigo-500 outline-none mt-1" rows="3"
						bind:value={editContent}
						onkeydown={(e) => { if (e.key === 'Escape') cancelEdit(); else if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); submitEdit(); } }}
					></textarea>
					<div class="flex gap-2 mt-1 text-xs">
						<button class="text-indigo-400 hover:text-indigo-300" onclick={submitEdit}>Save</button>
						<button class="text-gray-500 hover:text-gray-300" onclick={cancelEdit}>Cancel</button>
						<span class="text-gray-600">Enter to save · Esc to cancel</span>
					</div>
				{:else}
					<div class="text-gray-300 text-sm break-words leading-relaxed prose-sm">
						<MarkdownContent content={msg.content} />
					</div>
				{/if}

				<!-- Attachments -->
				{#if msg.attachments && msg.attachments.length > 0}
					<div class="flex flex-wrap gap-2 mt-2">
						{#each msg.attachments as att (att.id)}
							{#if isEncrypted(att.filename)}
								<!-- E2EE encrypted attachment: decrypt on demand -->
								<button
									onclick={() => decryptAndDownload(att.url, att.filename)}
									class="flex items-center gap-2 px-3 py-2 bg-gray-700/50 border border-indigo-600/50 rounded hover:border-indigo-400/50 transition-colors text-sm text-gray-300 hover:text-white"
									title="Click to decrypt and download"
								>
									<span>🔐</span>
									<span class="truncate max-w-xs">{originalName(att.filename)}</span>
									<span class="text-xs text-indigo-400 whitespace-nowrap">E2EE</span>
									<span class="text-xs text-gray-500 whitespace-nowrap">{formatFileSize(att.size)}</span>
								</button>
							{:else if att.content_type?.startsWith('image/')}
								<a href={att.url} target="_blank" rel="noopener noreferrer">
									<img
										src={att.url}
										alt={att.filename}
										class="max-h-64 max-w-sm rounded border border-gray-600 hover:border-gray-400 transition-colors object-contain"
										loading="lazy"
									/>
								</a>
							{:else if att.content_type?.startsWith('video/')}
								<video
									src={att.url}
									controls
									class="max-h-64 max-w-sm rounded border border-gray-600"
									preload="metadata"
								>
									<a href={att.url} download={att.filename}>{att.filename}</a>
								</video>
							{:else if att.content_type?.startsWith('audio/')}
								<div class="flex flex-col gap-1">
									<audio src={att.url} controls class="max-w-sm rounded"></audio>
									<span class="text-xs text-gray-500">{att.filename} · {formatFileSize(att.size)}</span>
								</div>
							{:else}
								<a
									href={att.url}
									target="_blank"
									rel="noopener noreferrer"
									download={att.filename}
									class="flex items-center gap-2 px-3 py-2 bg-gray-700/50 border border-gray-600/50 rounded hover:border-gray-400/50 transition-colors text-sm text-gray-300 hover:text-white"
								>
									<span>📎</span>
									<span class="truncate max-w-xs">{att.filename}</span>
									<span class="text-xs text-gray-500 whitespace-nowrap">{formatFileSize(att.size)}</span>
								</a>
							{/if}
						{/each}
					</div>
				{/if}

				<!-- Reactions display -->
				{#if (msg.reactions && msg.reactions.length > 0) || onReact}
					<div class="flex flex-wrap gap-1 mt-1 items-center">
						{#each (msg.reactions ?? []) as r (r.emoji)}
							<button
								class="flex items-center gap-1 px-1.5 py-0.5 rounded text-xs {r.reacted ? 'bg-indigo-900/50 border border-indigo-600/50' : 'bg-gray-700/50 border border-gray-600/30'} hover:border-indigo-500/50 transition-colors"
								onclick={() => onReact?.(msg.id, r.emoji, r.reacted)}
								title="{r.reacted ? 'Remove reaction' : 'Add reaction'}"
							>
								<span>{r.emoji}</span>
								<span class="text-gray-300">{r.count}</span>
							</button>
						{/each}

						<!-- Add reaction button -->
						{#if onReact}
							<div class="relative">
								<button
									class="opacity-0 group-hover:opacity-100 transition-opacity px-1.5 py-0.5 rounded text-xs bg-gray-700/50 border border-gray-600/30 hover:border-indigo-500/50 text-gray-400 hover:text-gray-200"
									onclick={() => toggleEmojiPicker(msg.id)}
									title="Add reaction"
								>+😀</button>
								{#if emojiPickerMsgId === msg.id}
									<div class="absolute bottom-7 left-0 bg-gray-800 border border-gray-600 rounded shadow-lg flex gap-1 p-1.5 z-10 flex-wrap max-w-xs">
										{#each quickEmojis as emoji}
											<button
												class="text-lg hover:scale-125 transition-transform"
												onclick={() => { onReact?.(msg.id, emoji, false); emojiPickerMsgId = null; }}
											>{emoji}</button>
										{/each}
										<button
											class="text-lg hover:scale-125 transition-transform text-gray-500 hover:text-gray-300"
											onclick={() => openFullPicker(msg.id)}
											title="More emojis"
										>+</button>
									</div>
								{/if}
							</div>
						{/if}
					</div>
				{/if}

				<!-- Full emoji picker for reactions -->
				{#if fullPickerMsgId === msg.id && onReact}
					<EmojiPicker
						onSelect={handleFullEmojiSelect}
						onClose={() => (fullPickerMsgId = null)}
					/>
				{/if}
			</div>

			<MessageContextMenu {msg} {currentUserId} {serverId} {onReply} {onPin} {onOpenThread} {onDelete}
				onStartEdit={startEdit} />
		</div>
	{/each}
</div>

{#if popoverUserId && popoverAnchorRect}
	<UserProfilePopover
		userId={popoverUserId}
		{serverId}
		anchorRect={popoverAnchorRect}
		onClose={() => { popoverUserId = null; popoverAnchorRect = null; }}
	/>
{/if}


<style>
	:global(.highlight-flash) {
		animation: msg-flash 1.5s ease-out;
	}
	@keyframes msg-flash {
		0%   { background-color: rgba(88, 101, 242, 0.25); }
		100% { background-color: transparent; }
	}
</style>
