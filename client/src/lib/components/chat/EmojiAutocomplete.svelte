<!--
  @file EmojiAutocomplete.svelte
  @purpose Inline emoji autocomplete dropdown for message input. Matches :shortcode against standard and custom emojis.
  @version 1.0.0
-->
<script lang="ts">
	interface EmojiOption {
		id: string;
		name: string;
		native?: string;
		imageUrl?: string;
		custom?: boolean;
	}

	interface Props {
		options: EmojiOption[];
		selectedIndex: number;
		onSelect: (option: EmojiOption) => void;
	}

	let { options, selectedIndex, onSelect }: Props = $props();
</script>

<div class="emoji-autocomplete">
	{#each options as opt, i (opt.id + (opt.custom ? '-custom' : ''))}
		<button
			type="button"
			class="emoji-option {i === selectedIndex ? 'selected' : ''}"
			onclick={() => onSelect(opt)}
		>
			{#if opt.imageUrl}
				<img src={opt.imageUrl} alt={opt.name} class="emoji-thumb" />
			{:else}
				<span class="emoji-native">{opt.native}</span>
			{/if}
			<span class="emoji-name">:{opt.name}:</span>
		</button>
	{/each}
</div>

<style>
	.emoji-autocomplete {
		position: absolute;
		bottom: 100%;
		left: 0;
		background: #2a2d31;
		border: 1px solid #35373c;
		border-bottom: none;
		border-radius: 8px 8px 0 0;
		max-height: 240px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		z-index: 10;
		min-width: 220px;
	}

	.emoji-option {
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

	.emoji-option:last-child {
		border-bottom: none;
	}

	.emoji-option:hover,
	.emoji-option.selected {
		background: #35373c;
	}

	.emoji-native {
		font-size: 20px;
		line-height: 1;
		width: 24px;
		text-align: center;
	}

	.emoji-thumb {
		width: 20px;
		height: 20px;
		object-fit: contain;
		border-radius: 2px;
	}

	.emoji-name {
		color: #b5bac1;
		font-size: 13px;
		font-family: 'Consolas', 'Monaco', monospace;
	}
</style>
