<!--
  @component EmojiGrid
  @purpose Display grid of custom emojis with delete buttons
  @version 1.0.0
-->
<script lang="ts">
	interface Emoji {
		id: string;
		name: string;
		image_url: string;
	}

	interface Props {
		emojis: Emoji[];
		onDelete: (id: string, name: string) => void;
	}

	let { emojis, onDelete }: Props = $props();
</script>

<div class="grid">
	{#each emojis as emoji (emoji.id)}
		<div class="card">
			<img src={emoji.image_url} alt={emoji.name} class="emoji-img" />
			<span class="label">{emoji.name}</span>
			<button
				class="del"
				onclick={() => onDelete(emoji.id, emoji.name)}
				aria-label="Delete emoji"
			>
				×
			</button>
		</div>
	{/each}
	{#if emojis.length === 0}
		<p class="empty">No custom emojis yet.</p>
	{/if}
</div>

<style>
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
		gap: 12px;
		min-height: 120px;
	}

	.card {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
		padding: 12px;
		background: #1e1f22;
		border-radius: 6px;
		transition: background-color 0.15s;
	}

	.card:hover {
		background: #2f3136;
	}

	.emoji-img {
		width: 64px;
		height: 64px;
		object-fit: contain;
		image-rendering: pixelated;
	}

	.label {
		font-size: 12px;
		color: #b5bac1;
		text-align: center;
		word-break: break-word;
		max-width: 100%;
	}

	.del {
		position: absolute;
		top: 4px;
		right: 4px;
		width: 20px;
		height: 20px;
		padding: 0;
		background: #ed4245;
		border: none;
		border-radius: 4px;
		color: white;
		cursor: pointer;
		font-size: 14px;
		opacity: 0;
		transition: opacity 0.15s, background-color 0.15s;
	}

	.card:hover .del {
		opacity: 1;
	}

	.del:hover {
		background: #c41e3a;
	}

	.empty {
		grid-column: 1 / -1;
		color: #b5bac1;
		font-size: 13px;
		text-align: center;
		padding: 24px;
		margin: 0;
	}
</style>
