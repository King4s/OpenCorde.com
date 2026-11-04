<!--
  @file MentionAutocomplete.svelte
  @purpose Inline mention autocomplete dropdown for message input. Matches @ against users, roles, and channels.
  @version 1.0.0
-->
<script lang="ts">
	import type { MentionOption } from './mentionUtils';

	interface Props {
		options: MentionOption[];
		selectedIndex: number;
		onSelect: (option: MentionOption) => void;
	}

	let { options, selectedIndex, onSelect }: Props = $props();

	function hexFromColor(color: number | null | undefined): string {
		if (!color) return '#b5bac1';
		return '#' + color.toString(16).padStart(6, '0');
	}
</script>

<div class="mention-autocomplete">
	{#each options as opt, i (opt.type + ':' + opt.id)}
		<button
			type="button"
			class="mention-option {i === selectedIndex ? 'selected' : ''}"
			onclick={() => onSelect(opt)}
		>
			{#if opt.type === 'user'}
				{#if opt.avatarUrl}
					<img src={opt.avatarUrl} alt="" class="mention-avatar" />
				{:else}
					<div class="mention-avatar-placeholder">{opt.name[0]?.toUpperCase() ?? '?'}</div>
				{/if}
				<span class="mention-name">{opt.displayName ?? opt.name}</span>
				<span class="mention-detail">@{opt.name}</span>
			{:else if opt.type === 'role'}
				<span class="mention-dot" style="background-color: {hexFromColor(opt.color)};"></span>
				<span class="mention-name" style="color: {hexFromColor(opt.color)};">{opt.name}</span>
				{#if !opt.mentionable}
					<span class="mention-warn" title="This role is not mentionable">🚫</span>
				{/if}
			{:else if opt.type === 'channel'}
				<span class="mention-hash">#</span>
				<span class="mention-name">{opt.name}</span>
			{:else if opt.type === 'special'}
				<span class="mention-hash">@</span>
				<span class="mention-name">{opt.name}</span>
			{/if}
		</button>
	{/each}
</div>

<style>
	.mention-autocomplete {
		position: absolute;
		bottom: 100%;
		left: 0;
		background: #2a2d31;
		border: 1px solid #35373c;
		border-bottom: none;
		border-radius: 8px 8px 0 0;
		max-height: 320px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		z-index: 10;
		min-width: 260px;
		max-width: 360px;
	}

	.mention-option {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 10px;
		border: none;
		background: none;
		color: #f2f3f5;
		cursor: pointer;
		text-align: left;
		border-bottom: 1px solid #35373c;
		transition: background-color 0.1s;
		font-size: 14px;
	}

	.mention-option:last-child {
		border-bottom: none;
	}

	.mention-option:hover,
	.mention-option.selected {
		background: #35373c;
	}

	.mention-avatar {
		width: 20px;
		height: 20px;
		border-radius: 50%;
		object-fit: cover;
		flex-shrink: 0;
	}

	.mention-avatar-placeholder {
		width: 20px;
		height: 20px;
		border-radius: 50%;
		background: #5865f2;
		color: #fff;
		font-size: 10px;
		font-weight: 700;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.mention-name {
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.mention-detail {
		color: #949ba4;
		font-size: 12px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.mention-dot {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.mention-hash {
		color: #949ba4;
		font-size: 16px;
		line-height: 1;
		flex-shrink: 0;
		width: 20px;
		text-align: center;
	}

	.mention-warn {
		font-size: 10px;
		margin-left: auto;
		flex-shrink: 0;
	}
</style>
