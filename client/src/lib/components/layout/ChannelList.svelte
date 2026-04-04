<!--
  @component ChannelList
  @purpose Renders channel list grouped by category with collapse toggle
  @version 2.0.0 — category collapsing
  @uses Svelte 5 $state(), $derived()
-->
<script lang="ts">
	import { channels, selectChannel, currentChannelId } from '$lib/stores/channels';
	import { currentVoiceChannelId, joinVoice } from '$lib/stores/voice';
	import { unreadCounts } from '$lib/stores/unread';
	import { notifSettings } from '$lib/stores/notificationSettings';
	import { fetchStage } from '$lib/stores/stage';
	import type { Channel } from '$lib/api/types';

	interface Props {
		spaceId: string;
		onDragStart: (e: DragEvent, channelId: string, channelType: number) => void;
		onDragOver: (e: DragEvent) => void;
		onDrop: (e: DragEvent, targetChannelId: string, targetChannelType: number) => void;
		onDragEnd: () => void;
		onContextMenu: (e: MouseEvent, channel: { id: string; name: string }) => void;
	}

	let { spaceId, onDragStart, onDragOver, onDrop, onDragEnd, onContextMenu }: Props = $props();

	// Set of category IDs the user has collapsed
	let collapsedCategories = $state<Set<string>>(new Set());

	function toggleCategory(categoryId: string) {
		collapsedCategories = new Set(
			collapsedCategories.has(categoryId)
				? [...collapsedCategories].filter(id => id !== categoryId)
				: [...collapsedCategories, categoryId]
		);
	}

	// Channels with no parent_id (root level), grouped by type
	const rootText  = $derived($channels.filter(c => (c.channel_type === 0 || c.channel_type === 4) && !c.parent_id));
	const rootForum = $derived($channels.filter(c => c.channel_type === 5 && !c.parent_id));
	const rootVoice = $derived($channels.filter(c => c.channel_type === 1 && !c.parent_id));
	const rootStage = $derived($channels.filter(c => c.channel_type === 3 && !c.parent_id));

	function childrenOf(cat: Channel): Channel[] {
		return $channels
			.filter(c => c.parent_id === cat.id)
			.sort((a, b) => a.position - b.position);
	}

	function channelIcon(ch: Channel): string {
		if (ch.channel_type === 1) return '🔊';
		if (ch.channel_type === 3) return '🎙️';
		if (ch.channel_type === 5) return '🗂️';
		if (ch.channel_type === 4) return '📢';
		return '#';
	}

	function unread(channelId: string): number {
		return $unreadCounts.get(channelId) ?? 0;
	}

	function channelUrl(ch: Channel): string {
		return ch.channel_type === 5
			? `/servers/${spaceId}/forum/${ch.id}`
			: `/servers/${spaceId}/channels/${ch.id}`;
	}

	function isTextLike(ch: Channel): boolean {
		return ch.channel_type === 0 || ch.channel_type === 4;
	}
</script>

<!-- Root text channels (no category) -->
{#each rootText as channel (channel.id)}
	{@const u = unread(channel.id)}
	<a
		href={channelUrl(channel)}
		draggable={true}
		ondragstart={(e) => onDragStart(e, channel.id, channel.channel_type)}
		ondragover={onDragOver}
		ondrop={(e) => onDrop(e, channel.id, channel.channel_type)}
		ondragend={onDragEnd}
		class="w-full px-2 py-1.5 rounded text-left text-sm transition-colors flex items-center justify-between group {$currentChannelId === channel.id ? 'bg-gray-700 text-white' : 'text-gray-400 hover:text-gray-200 hover:bg-gray-700/50'}"
		onclick={() => selectChannel(channel.id)}
		oncontextmenu={(e) => onContextMenu(e, channel)}
	>
		<div class="flex items-center min-w-0 gap-1">
			<span class="text-gray-500 text-xs">{channelIcon(channel)}</span>
			<span class={u > 0 ? 'font-semibold text-white' : ''}>{channel.name}</span>
			{#if channel.e2ee_enabled}
				<span class="text-gray-500 text-xs" title="End-to-End Encrypted">🔒</span>
			{/if}
			{#if ($notifSettings.get(channel.id) ?? 0) === 2}
				<span class="ml-auto text-gray-600 text-xs" title="Muted">🔕</span>
			{:else if ($notifSettings.get(channel.id) ?? 0) === 1}
				<span class="ml-auto text-gray-600 text-xs" title="Mentions only">🔔</span>
			{/if}
			{#if channel.nsfw}
				<span style="font-size:9px;background:#ed4245;color:white;padding:1px 4px;border-radius:3px;font-weight:700;">NSFW</span>
			{/if}
		</div>
		{#if u > 0}
			<span class="ml-2 px-1.5 py-0.5 rounded bg-gray-600 text-white text-xs font-semibold">{u}</span>
		{/if}
	</a>
{/each}

<!-- Forum channels (standalone type-2 rows without children) -->
{#if rootForum.length > 0}
	<div class="mt-2">
		<h3 class="text-xs font-semibold text-gray-400 uppercase px-2 mb-1">Forum Channels</h3>
		{#each rootForum as channel (channel.id)}
			{@const u = unread(channel.id)}
			<a
				href={channelUrl(channel)}
				draggable={true}
				ondragstart={(e) => onDragStart(e, channel.id, channel.channel_type)}
				ondragover={onDragOver}
				ondrop={(e) => onDrop(e, channel.id, channel.channel_type)}
				ondragend={onDragEnd}
				class="w-full px-2 py-1.5 rounded text-left text-sm transition-colors flex items-center justify-between group {$currentChannelId === channel.id ? 'bg-gray-700 text-white' : 'text-gray-400 hover:text-gray-200 hover:bg-gray-700/50'}"
				onclick={() => selectChannel(channel.id)}
				oncontextmenu={(e) => onContextMenu(e, channel)}
			>
				<div class="flex items-center gap-1 min-w-0">
					<span class="text-gray-500 text-xs">{channelIcon(channel)}</span>
					<span class={u > 0 ? 'font-semibold text-white' : ''}>{channel.name}</span>
				</div>
				{#if u > 0}
					<span class="ml-2 px-1.5 py-0.5 rounded bg-gray-600 text-white text-xs font-semibold">{u}</span>
				{/if}
			</a>
		{/each}
	</div>
{/if}

<!-- Categories with their children -->
{#each $channels.filter(c => c.channel_type === 2) as cat (cat.id)}
	{@const children = childrenOf(cat)}
	{@const collapsed = collapsedCategories.has(cat.id)}
	{@const catUnread = children.reduce((sum, c) => sum + unread(c.id), 0)}

	<button
		class="w-full flex items-center gap-1 px-1 py-1 mt-1 text-xs font-semibold text-gray-400 uppercase hover:text-gray-200 transition-colors"
		onclick={() => toggleCategory(cat.id)}
		title={collapsed ? 'Expand' : 'Collapse'}
	>
		<span class="transition-transform {collapsed ? '-rotate-90' : ''}">▾</span>
		<span class="flex-1 text-left truncate">{cat.name}</span>
		{#if catUnread > 0 && collapsed}
			<span class="w-2 h-2 rounded-full bg-gray-500 flex-shrink-0"></span>
		{/if}
	</button>

	{#if !collapsed}
		{#each children as channel (channel.id)}
			{@const u = unread(channel.id)}
			{#if isTextLike(channel)}
				<a
					href={channelUrl(channel)}
					draggable={true}
					ondragstart={(e) => onDragStart(e, channel.id, channel.channel_type)}
					ondragover={onDragOver}
					ondrop={(e) => onDrop(e, channel.id, channel.channel_type)}
					ondragend={onDragEnd}
					class="w-full pl-4 pr-2 py-1 rounded text-left text-sm transition-colors flex items-center justify-between {$currentChannelId === channel.id ? 'bg-gray-700 text-white' : 'text-gray-400 hover:text-gray-200 hover:bg-gray-700/50'}"
					onclick={() => selectChannel(channel.id)}
					oncontextmenu={(e) => onContextMenu(e, channel)}
				>
					<div class="flex items-center gap-1 min-w-0">
						<span class="text-gray-500 text-xs">{channelIcon(channel)}</span>
						<span class={u > 0 ? 'font-semibold text-white' : ''}>{channel.name}</span>
						{#if channel.e2ee_enabled}
							<span class="text-gray-500 text-xs" title="End-to-End Encrypted">🔒</span>
						{/if}
						{#if ($notifSettings.get(channel.id) ?? 0) === 2}
							<span class="ml-auto text-gray-600 text-xs" title="Muted">🔕</span>
						{:else if ($notifSettings.get(channel.id) ?? 0) === 1}
							<span class="ml-auto text-gray-600 text-xs" title="Mentions only">🔔</span>
						{/if}
						{#if channel.nsfw}
							<span style="font-size:9px;background:#ed4245;color:white;padding:1px 4px;border-radius:3px;font-weight:700;">NSFW</span>
						{/if}
					</div>
					{#if u > 0}
						<span class="ml-2 px-1.5 py-0.5 rounded bg-gray-600 text-white text-xs font-semibold">{u}</span>
					{/if}
				</a>
			{:else}
				<button
					draggable={true}
					ondragstart={(e) => onDragStart(e, channel.id, channel.channel_type)}
					ondragover={onDragOver}
					ondrop={(e) => onDrop(e, channel.id, channel.channel_type)}
					ondragend={onDragEnd}
					class="w-full pl-4 pr-2 py-1 rounded text-left text-sm transition-colors {$currentVoiceChannelId === channel.id ? 'bg-gray-700 text-white' : 'text-gray-400 hover:text-gray-200 hover:bg-gray-700/50'}"
					onclick={() => joinVoice(channel.id)}
					oncontextmenu={(e) => onContextMenu(e, channel)}
				>
					<span class="text-gray-500 mr-1">{channel.channel_type === 3 ? '🎙️' : '🔊'}</span>{channel.name}
				</button>
			{/if}
		{/each}
	{/if}
{/each}

<!-- Root voice channels -->
{#if rootVoice.length > 0}
	<div class="mt-2">
		<h3 class="text-xs font-semibold text-gray-400 uppercase px-2 mb-1">Voice Channels</h3>
		{#each rootVoice as channel (channel.id)}
			<button
				draggable={true}
				ondragstart={(e) => onDragStart(e, channel.id, 1)}
				ondragover={onDragOver}
				ondrop={(e) => onDrop(e, channel.id, 1)}
				ondragend={onDragEnd}
				class="w-full px-2 py-1.5 rounded text-left text-sm transition-colors {$currentVoiceChannelId === channel.id ? 'bg-gray-700 text-white' : 'text-gray-400 hover:text-gray-200 hover:bg-gray-700/50'}"
				onclick={() => joinVoice(channel.id)}
				oncontextmenu={(e) => onContextMenu(e, channel)}
			>
				<span class="text-gray-500 mr-1">🔊</span>{channel.name}
			</button>
		{/each}
	</div>
{/if}

{#if rootStage.length > 0}
	<div class="mt-2">
		<h3 class="text-xs font-semibold text-gray-400 uppercase px-2 mb-1">Stage Channels</h3>
		{#each rootStage as channel (channel.id)}
			<button
				draggable={true}
				ondragstart={(e) => onDragStart(e, channel.id, 3)}
				ondragover={onDragOver}
				ondrop={(e) => onDrop(e, channel.id, 3)}
				ondragend={onDragEnd}
				class="w-full px-2 py-1.5 rounded text-left text-sm transition-colors {$currentChannelId === channel.id ? 'bg-gray-700 text-white' : 'text-gray-400 hover:text-gray-200 hover:bg-gray-700/50'}"
				onclick={() => {
					selectChannel(channel.id);
					fetchStage(channel.id).catch(() => {});
				}}
				oncontextmenu={(e) => onContextMenu(e, channel)}
			>
				<span class="text-gray-500 mr-1">🎙️</span>{channel.name}
			</button>
		{/each}
	</div>
{/if}
