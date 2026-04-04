<script lang="ts">
	/**
	 * @file Message input component
	 * @purpose Text input + send button, file upload, typing notifications, reply mode, emoji picker
	 * @version 3.1.0
	 */
	import { sendTyping } from '$lib/stores/typing';
	import { slashCommandsStore } from '$lib/stores/slashCommands.svelte';
	import type { Message, Attachment } from '$lib/api/types';
	import api from '$lib/api/client';
	import EmojiPicker from './EmojiPicker.svelte';
	import CommandAutocomplete from './CommandAutocomplete.svelte';
	import AttachmentPreview from './AttachmentPreview.svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { getGroupState } from '$lib/stores/e2ee';

	interface Props {
		onSend: (content: string, replyToId?: string, attachments?: Attachment[]) => void;
		channelName: string;
		channelId: string;
		replyTo?: Message | null;
		onCancelReply?: () => void;
		onEditLast?: () => void;
	}

	let { onSend, channelName, channelId, replyTo = null, onCancelReply, onEditLast }: Props = $props();
	let content = $state('');
	let inputElement: HTMLInputElement;
	let fileInputElement: HTMLInputElement;
	let pendingAttachments = $state<Attachment[]>([]);
	let uploading = $state(false);
	let uploadError = $state('');
	let showEmojiPicker = $state(false);
	let showCommandAutocomplete = $state(false);
	let commandPrefix = $state('');
	let dispatchingCommand = $state(false);

	function getMatchingCommands(prefix: string) {
		return slashCommandsStore.commands.filter(c =>
			c.name.toLowerCase().startsWith(prefix.toLowerCase())
		);
	}

	async function handleSubmit(e: Event) {
		e.preventDefault();
		if (!content.trim() && pendingAttachments.length === 0) return;

		// Check if message is a slash command
		const trimmed = content.trim();
		if (trimmed.startsWith('/')) {
			const parts = trimmed.split(/\s+/);
			const commandName = parts[0].slice(1).toLowerCase();
			const args = parts.slice(1);

			const command = slashCommandsStore.commands.find(c => c.name === commandName);
			if (command) {
				dispatchingCommand = true;
				uploadError = '';
				try {
					await slashCommandsStore.dispatchCommand(channelId, '/' + commandName, args);
					content = '';
					inputElement?.focus();
				} catch (err: any) {
					uploadError = err.message ?? 'Command failed';
				} finally {
					dispatchingCommand = false;
				}
				return;
			} else if (content.length > 1) {
				uploadError = `Command not found: ${trimmed}`;
				return;
			}
		}

		// Regular message
		onSend(content.trim(), replyTo?.id, pendingAttachments.length > 0 ? [...pendingAttachments] : undefined);
		content = '';
		pendingAttachments = [];
		uploadError = '';
		inputElement?.focus();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			handleSubmit(e);
		} else if (e.key === 'Escape') {
			if (showCommandAutocomplete) {
				showCommandAutocomplete = false;
			} else if (replyTo) {
				onCancelReply?.();
			}
		} else if (e.key === 'ArrowUp' && !content.trim() && !replyTo) {
			onEditLast?.();
		} else if (content.length > 0 || e.key.length === 1) {
			// Update command autocomplete
			const trimmed = content.trim();
			if (trimmed.startsWith('/') && !trimmed.includes(' ')) {
				commandPrefix = trimmed.slice(1);
				showCommandAutocomplete = getMatchingCommands(commandPrefix).length > 0;
			} else {
				showCommandAutocomplete = false;
			}
			sendTyping(channelId);
		}
	}

	function selectCommand(name: string) {
		content = '/' + name + ' ';
		showCommandAutocomplete = false;
		inputElement?.focus();
	}

	/** Convert ArrayBuffer to base64 string for Tauri IPC. */
	function arrayBufferToBase64(buf: ArrayBuffer): string {
		const bytes = new Uint8Array(buf);
		let binary = '';
		for (const b of bytes) binary += String.fromCharCode(b);
		return btoa(binary);
	}

	/** Convert base64 string back to Uint8Array. */
	function base64ToUint8Array(b64: string): Uint8Array {
		const binary = atob(b64);
		const arr = new Uint8Array(binary.length);
		for (let i = 0; i < binary.length; i++) arr[i] = binary.charCodeAt(i);
		return arr;
	}

	async function handleFilesSelected(e: Event) {
		const input = e.target as HTMLInputElement;
		if (!input.files || input.files.length === 0) return;

		uploading = true;
		uploadError = '';
		const groupState = getGroupState(channelId);

		for (const file of Array.from(input.files)) {
			try {
				let uploadFile: File = file;

				// Encrypt file if channel has an active E2EE group
				if (groupState) {
					const fileBuffer = await file.arrayBuffer();
					const fileB64 = arrayBufferToBase64(fileBuffer);
					const encryptedB64 = await invoke<string>('crypto_encrypt_file', {
						group_state_hex: groupState,
						file_data_b64: fileB64
					});
					const encryptedBytes = base64ToUint8Array(encryptedB64);
					// Slice to get a plain ArrayBuffer (Uint8Array.buffer is ArrayBufferLike)
					const encBuf = encryptedBytes.buffer.slice(
						encryptedBytes.byteOffset,
						encryptedBytes.byteOffset + encryptedBytes.byteLength
					) as ArrayBuffer;
					// Mark as encrypted: append .enc extension, octet-stream MIME
					uploadFile = new File([encBuf], file.name + '.enc', {
						type: 'application/octet-stream'
					});
				}

				const formData = new FormData();
				formData.append('file', uploadFile);
				const attachment = await api.postFormData<Attachment>(
					`/channels/${channelId}/attachments`,
					formData
				);
				pendingAttachments = [...pendingAttachments, attachment];
			} catch (err: any) {
				uploadError = err.message ?? 'Upload failed';
			}
		}

		uploading = false;
		// Reset input so same file can be selected again
		input.value = '';
	}

	function removeAttachment(id: string) {
		pendingAttachments = pendingAttachments.filter(a => a.id !== id);
	}

	function handleEmojiSelect(emoji: string) {
		content += emoji;
		showEmojiPicker = false;
		inputElement?.focus();
	}
</script>

<div class="px-4 pb-3">
	<!-- Reply indicator -->
	{#if replyTo}
		<div class="flex items-center justify-between px-3 py-1.5 mb-1 bg-gray-700/50 rounded-t-lg border-b border-gray-600/30 text-xs text-gray-400">
			<span>↩ Replying to <span class="text-gray-400 font-medium">{replyTo.author_username}</span></span>
			<button onclick={onCancelReply} class="text-gray-500 hover:text-gray-300 ml-2">✕</button>
		</div>
	{/if}

	<!-- Pending attachments preview -->
	{#if pendingAttachments.length > 0}
		<div class="mb-2">
			<AttachmentPreview
				attachments={pendingAttachments}
				onRemove={removeAttachment}
			/>
		</div>
	{/if}

	{#if uploadError}
		<p class="text-gray-400 text-xs px-3 py-1">{uploadError}</p>
	{/if}

	{#if showCommandAutocomplete}
		<CommandAutocomplete
			commands={getMatchingCommands(commandPrefix)}
			onSelect={selectCommand}
		/>
	{/if}

	<form onsubmit={handleSubmit}>
		<div class="flex items-center bg-gray-700 {replyTo || pendingAttachments.length > 0 || showCommandAutocomplete ? 'rounded-b-lg' : 'rounded-lg'} px-2 py-2">
			<!-- File upload button -->
			<button
				type="button"
				onclick={() => fileInputElement?.click()}
				disabled={uploading || dispatchingCommand}
				class="mr-1 w-8 h-8 flex items-center justify-center text-gray-400 hover:text-gray-200 disabled:text-gray-600 transition-colors rounded hover:bg-gray-600/50 flex-shrink-0"
				title="Attach file"
				aria-label="Attach file"
			>
				{#if uploading}
					<span class="text-xs animate-pulse">...</span>
				{:else}
					📎
				{/if}
			</button>

			<!-- Emoji picker button -->
			<button
				type="button"
				onclick={() => (showEmojiPicker = !showEmojiPicker)}
				class="mr-1 w-8 h-8 flex items-center justify-center text-gray-400 hover:text-gray-200 transition-colors rounded hover:bg-gray-600/50 flex-shrink-0"
				title="Add emoji"
				aria-label="Add emoji"
			>
				😊
			</button>

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
				disabled={(!content.trim() && pendingAttachments.length === 0) || dispatchingCommand}
				class="ml-2 text-gray-400 hover:text-gray-300 disabled:text-gray-600 transition-colors text-sm font-medium"
			>
				{dispatchingCommand ? 'Executing...' : 'Send'}
			</button>
		</div>
	</form>

	{#if showEmojiPicker}
		<EmojiPicker
			onSelect={handleEmojiSelect}
			onClose={() => (showEmojiPicker = false)}
		/>
	{/if}

	<!-- Hidden file input -->
	<input
		type="file"
		multiple
		bind:this={fileInputElement}
		onchange={handleFilesSelected}
		class="hidden"
		accept="image/*,application/pdf,.txt,.md,.csv,.json,.zip,.tar,.gz"
	/>
</div>
