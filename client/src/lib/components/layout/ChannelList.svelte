<!--
  @component ChannelList
  @purpose Renders grouped channel lists (text, voice, stage) with drag-drop
  @uses Svelte 5 $props() rune
-->
<script lang="ts">
	import { textChannels, voiceChannels, stageChannels, selectChannel, currentChannelId } from '$lib/stores/channels';
	import { currentVoiceChannelId, joinVoice } from '$lib/stores/voice';
	import { unreadCounts } from '$lib/stores/unread';
	import { fetchStage } from '$lib/stores/stage';

	interface Props {
		serverId: string;
		onDragStart: (e: DragEvent, channelId: string, channelType: number) => void;
		onDragOver: (e: DragEvent) => void;
		onDrop: (e: DragEvent, targetChannelId: string, targetChannelType: number) => void;
		onDragEnd: () => void;
		onContextMenu: (e: MouseEvent, channel: { id: string; name: string }) => void;
	}

	let { serverId, onDragStart, onDragOver, onDrop, onDragEnd, onContextMenu }: Props = $props();
</script>

{#if $textChannels.length > 0}
	<div>
		<h3 class="text-xs font-semibold text-gray-400 uppercase px-2 mb-1">Text Channels</h3>
		{#each $textChannels as channel (channel.id)}
			{@const unreadCount = $unreadCounts.get(channel.id) ?? 0}
			{@const isForum = channel.channel_type === 2}
			{@const channelUrl = isForum ? `/servers/${serverId}/forum/${channel.id}` : `/servers/${serverId}/channels/${channel.id}`}
			<a
				href={channelUrl}
				draggable={true}
				ondragstart={(e) => onDragStart(e, channel.id, 0)}
				ondragover={onDragOver}
				ondrop={(e) => onDrop(e, channel.id, 0)}
				ondragend={onDragEnd}
				class="w-full px-2 py-1.5 rounded text-left text-sm transition-colors flex items-center justify-between group {$currentChannelId === channel.id ? 'bg-gray-700 text-white' : 'text-gray-400 hover:text-gray-200 hover:bg-gray-700/50'}"
				onclick={() => selectChannel(channel.id)}
				oncontextmenu={(e) => onContextMenu(e, channel)}
			>
				<div class="flex items-center min-w-0 gap-1">
					<span class="text-gray-500">{isForum ? '💬' : '#'}</span>
					<span class={unreadCount > 0 ? 'font-semibold' : ''}>{channel.name}</span>
					{#if channel.nsfw}
						<span style="font-size: 9px; background: #ed4245; color: white; padding: 1px 4px; border-radius: 3px; font-weight: 700; letter-spacing: 0.5px;">NSFW</span>
					{/if}
				</div>
				{#if unreadCount > 0}
					<span class="ml-2 px-1.5 py-0.5 rounded bg-indigo-600 text-white text-xs font-semibold whitespace-nowrap">{unreadCount}</span>
				{/if}
			</a>
		{/each}
	</div>
{/if}

{#if $voiceChannels.length > 0}
	<div>
		<h3 class="text-xs font-semibold text-gray-400 uppercase px-2 mb-1">Voice Channels</h3>
		{#each $voiceChannels as channel (channel.id)}
			<button
				draggable={true}
				ondragstart={(e) => onDragStart(e, channel.id, 1)}
				ondragover={onDragOver}
				ondrop={(e) => onDrop(e, channel.id, 1)}
				ondragend={onDragEnd}
				class="w-full px-2 py-1.5 rounded text-left text-sm transition-colors {$currentVoiceChannelId === channel.id ? 'bg-green-700 text-white' : 'text-gray-400 hover:text-gray-200 hover:bg-gray-700/50'}"
				onclick={() => joinVoice(channel.id)}
				oncontextmenu={(e) => onContextMenu(e, channel)}
			>
				<span class="text-gray-500 mr-1">&#x1F50A;</span>{channel.name}
			</button>
		{/each}
	</div>
{/if}

{#if $stageChannels.length > 0}
	<div>
		<h3 class="text-xs font-semibold text-gray-400 uppercase px-2 mb-1">Stage Channels</h3>
		{#each $stageChannels as channel (channel.id)}
			<button
				draggable={true}
				ondragstart={(e) => onDragStart(e, channel.id, 3)}
				ondragover={onDragOver}
				ondrop={(e) => onDrop(e, channel.id, 3)}
				ondragend={onDragEnd}
				class="w-full px-2 py-1.5 rounded text-left text-sm transition-colors {$currentChannelId === channel.id ? 'bg-purple-700 text-white' : 'text-gray-400 hover:text-gray-200 hover:bg-gray-700/50'}"
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
