<!--
  @component ChannelSidebar
  @purpose Renders text/voice/stage channel lists with drag-drop, context menu, and create forms
  @uses Svelte 5 $props(), $state(), $derived() runes
-->
<script lang="ts">
	import { get } from 'svelte/store';
	import {
		fetchChannels,
		channels,
		textChannels,
		voiceChannels,
		stageChannels
	} from '$lib/stores/channels';
	import api from '$lib/api/client';
	import { notifSettings, setNotifLevel, fetchNotifSettings, NOTIF_LABELS, type NotifLevel } from '$lib/stores/notificationSettings';
	import ChannelList from './ChannelList.svelte';

	interface Props {
		serverId: string;
		showCreateChannel: boolean;
		onCreateChannelToggle: () => void;
		onWebhookOpen: (channelId: string) => void;
	}

	let {
		serverId,
		showCreateChannel,
		onCreateChannelToggle,
		onWebhookOpen
	}: Props = $props();

	let channelName = $state('');
	let channelType = $state(0);
	let channelNsfw = $state(false);
	let error = $state('');

	// Channel context menu
	let channelMenu = $state<{ visible: boolean; channelId: string; channelName: string; x: number; y: number } | null>(null);
	let editingChannelName = $state('');
	let channelMenuError = $state('');

	// Drag-and-drop
	let draggedChannelId = $state<string | null>(null);
	let draggedChannelType = $state<number | null>(null);

	async function handleCreateChannel() {
		if (!channelName.trim() || !serverId) return;
		error = '';
		try {
			await api.post(`/servers/${serverId}/channels`, {
				name: channelName.trim(),
				channel_type: channelType,
				nsfw: channelNsfw
			});
			await fetchChannels(serverId);
			onCreateChannelToggle();
			channelName = '';
			channelType = 0;
			channelNsfw = false;
		} catch (e: any) {
			error = e.message || 'Failed to create channel';
		}
	}

	// Load notification settings alongside channel list
	fetchNotifSettings();

	function openChannelMenu(e: MouseEvent, channel: { id: string; name: string }) {
		e.preventDefault();
		channelMenu = { visible: true, channelId: channel.id, channelName: channel.name, x: e.clientX, y: e.clientY };
		editingChannelName = channel.name;
		channelMenuError = '';
	}

	function closeChannelMenu() {
		channelMenu = null;
		channelMenuError = '';
	}

	async function handleRenameChannel() {
		if (!channelMenu || !editingChannelName.trim()) return;
		channelMenuError = '';
		try {
			await api.patch(`/channels/${channelMenu.channelId}`, { name: editingChannelName.trim() });
			await fetchChannels(serverId);
			closeChannelMenu();
		} catch (e: any) {
			channelMenuError = e.message ?? 'Failed to rename';
		}
	}

	async function handleDeleteChannel() {
		if (!channelMenu) return;
		if (!confirm(`Delete #${channelMenu.channelName}? This cannot be undone.`)) return;
		channelMenuError = '';
		try {
			await api.delete(`/channels/${channelMenu.channelId}`);
			await fetchChannels(serverId);
			closeChannelMenu();
		} catch (e: any) {
			channelMenuError = e.message ?? 'Failed to delete';
		}
	}

	function handleDragStart(e: DragEvent, channelId: string, channelType: number) {
		draggedChannelId = channelId;
		draggedChannelType = channelType;
		if (e.dataTransfer) {
			e.dataTransfer.effectAllowed = 'move';
		}
	}

	function handleDragOver(e: DragEvent) {
		e.preventDefault();
		if (e.dataTransfer) {
			e.dataTransfer.dropEffect = 'move';
		}
	}

	async function handleDrop(e: DragEvent, targetChannelId: string, targetChannelType: number) {
		e.preventDefault();
		if (!draggedChannelId || draggedChannelType !== targetChannelType) {
			draggedChannelId = null;
			draggedChannelType = null;
			return;
		}

		const allChannels = get(channels);
		const list = draggedChannelType === 0 ? get(textChannels) : draggedChannelType === 1 ? get(voiceChannels) : get(stageChannels);
		const draggedIdx = list.findIndex((c) => c.id === draggedChannelId);
		const targetIdx = list.findIndex((c) => c.id === targetChannelId);

		if (draggedIdx === -1 || targetIdx === -1) {
			draggedChannelId = null;
			draggedChannelType = null;
			return;
		}

		// Reorder optimistically by updating positions in the base channels store
		const newList = [...list];
		const [moved] = newList.splice(draggedIdx, 1);
		newList.splice(targetIdx, 0, moved);

		// Merge reordered list back into all channels
		const otherChannels = allChannels.filter(c => c.channel_type !== draggedChannelType);
		channels.set([...otherChannels, ...newList]);

		// Send to backend
		try {
			await api.patch(`/channels/${draggedChannelId}`, { position: targetIdx });
		} catch (e: any) {
			// Revert on error
			await fetchChannels(serverId);
		}

		draggedChannelId = null;
		draggedChannelType = null;
	}

	function handleDragEnd() {
		draggedChannelId = null;
		draggedChannelType = null;
	}
</script>

<!-- Create channel form -->
{#if showCreateChannel}
	<div class="p-2 bg-gray-750 border-b border-gray-900">
		{#if error}<p class="text-red-400 text-xs mb-1">{error}</p>{/if}
		<input type="text" bind:value={channelName} placeholder="channel-name"
			class="w-full px-2 py-1 bg-gray-900 border border-gray-700 rounded text-white text-xs placeholder-gray-500 focus:outline-none focus:border-indigo-500 mb-1" />
		<div class="flex gap-1 mb-1">
			<button onclick={() => channelType = 0}
				class="flex-1 text-xs py-1 rounded {channelType === 0 ? 'bg-indigo-600 text-white' : 'bg-gray-700 text-gray-400'}">Text</button>
			<button onclick={() => channelType = 1}
				class="flex-1 text-xs py-1 rounded {channelType === 1 ? 'bg-indigo-600 text-white' : 'bg-gray-700 text-gray-400'}">Voice</button>
			<button onclick={() => channelType = 3}
				class="flex-1 text-xs py-1 rounded {channelType === 3 ? 'bg-indigo-600 text-white' : 'bg-gray-700 text-gray-400'}">Stage</button>
		</div>
		<label class="flex items-center gap-2 mb-1 cursor-pointer">
			<input type="checkbox" bind:checked={channelNsfw} class="w-4 h-4" />
			<span class="text-xs text-gray-400">NSFW</span>
		</label>
		<button onclick={handleCreateChannel} disabled={!channelName.trim()}
			class="w-full text-xs py-1 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white rounded">Create</button>
	</div>
{/if}

<div class="flex-1 overflow-y-auto p-2 space-y-4">
	<ChannelList
		{serverId}
		onDragStart={handleDragStart}
		onDragOver={handleDragOver}
		onDrop={handleDrop}
		onDragEnd={handleDragEnd}
		onContextMenu={openChannelMenu}
	/>
</div>

{#if channelMenu?.visible}
	<!-- Backdrop -->
	<div class="fixed inset-0 z-40" role="button" tabindex="-1" aria-label="Close menu" onclick={closeChannelMenu} onkeydown={(e) => e.key === 'Escape' && closeChannelMenu()}></div>
	<!-- Menu -->
	<div class="fixed z-50 bg-gray-900 border border-gray-700 rounded-lg shadow-xl py-1 min-w-44" style="left: {channelMenu.x}px; top: {channelMenu.y}px">
		<div class="px-3 py-1.5 text-xs text-gray-400 font-semibold uppercase border-b border-gray-700 truncate">
			#{channelMenu.channelName}
		</div>
		{#if channelMenuError}
			<p class="px-3 py-1 text-xs text-red-400">{channelMenuError}</p>
		{/if}
		<div class="px-3 py-2 border-b border-gray-700">
			<label class="block text-xs text-gray-500 mb-1" for="rename-channel-input">Rename</label>
			<div class="flex gap-1">
				<input
					id="rename-channel-input"
					type="text"
					bind:value={editingChannelName}
					class="flex-1 min-w-0 px-2 py-1 bg-gray-800 text-white text-xs rounded border border-gray-700 focus:border-indigo-500 outline-none"
					onkeydown={(e) => e.key === 'Enter' && handleRenameChannel()}
				/>
				<button onclick={handleRenameChannel} class="px-2 py-1 bg-indigo-700 hover:bg-indigo-600 text-white text-xs rounded">OK</button>
			</div>
		</div>
		<!-- Notification level picker -->
		<div class="px-3 py-2 border-b border-gray-700">
			<p class="text-xs text-gray-500 mb-1.5 font-semibold uppercase">Notifications</p>
			<div class="flex flex-col gap-0.5">
				{#each ([0, 1, 2] as const) as lvl (lvl)}
					{@const active = ($notifSettings.get(channelMenu?.channelId ?? '') ?? 0) === lvl}
					<button
						class="text-left px-2 py-1 rounded text-xs transition-colors {active ? 'bg-indigo-600 text-white' : 'text-gray-400 hover:bg-gray-800'}"
						onclick={async () => { if (channelMenu) { await setNotifLevel(channelMenu.channelId, lvl as NotifLevel); } }}
					>{NOTIF_LABELS[lvl]}</button>
				{/each}
			</div>
		</div>
		<button
			class="w-full text-left px-3 py-1.5 text-sm text-blue-400 hover:bg-gray-800 transition-colors"
			onclick={() => { onWebhookOpen(channelMenu?.channelId ?? ''); channelMenu = null; }}
		>Webhooks</button>
		<button
			class="w-full text-left px-3 py-1.5 text-sm text-red-400 hover:bg-gray-800 transition-colors"
			onclick={handleDeleteChannel}
		>Delete Channel</button>
	</div>
{/if}

<style>
	/* Modal and drag styles are handled inline with Tailwind */
</style>
