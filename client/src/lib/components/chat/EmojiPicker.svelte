<!--
  @file EmojiPicker.svelte
  @purpose Emoji picker using emoji-mart. Emits selected emoji via callback.
  @version 1.0.0
-->
<script lang="ts">
	import { onMount } from 'svelte';

	interface Props {
		onSelect: (emoji: string) => void;
		onClose: () => void;
	}

	let { onSelect, onClose }: Props = $props();

	let container: HTMLDivElement;
	let isLoading = $state(true);

	onMount(async () => {
		try {
			// Dynamic import to avoid SSR issues
			const [EmojiMartModule, dataModule] = await Promise.all([
				import('emoji-mart'),
				import('@emoji-mart/data')
			]);

			const Picker = EmojiMartModule.Picker;
			const data = dataModule.default;

			const picker = new Picker({
				data,
				onEmojiSelect: (emoji: { native: string }) => {
					onSelect(emoji.native);
					onClose();
				},
				theme: 'dark',
				set: 'native',
				previewPosition: 'none',
				skinTonePosition: 'none',
				perLine: 8,
				maxFrequentRows: 1
			});

			if (container) {
				container.appendChild(picker as unknown as Node);
			}
		} catch (err) {
			console.error('Failed to load emoji picker:', err);
		} finally {
			isLoading = false;
		}
	});

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) onClose();
	}
</script>

<div class="picker-backdrop" onclick={handleBackdropClick} role="presentation" aria-label="Emoji picker backdrop">
	<div class="picker-container" role="dialog" aria-label="Emoji picker">
		{#if isLoading}
			<div class="picker-loading">Loading emojis...</div>
		{/if}
		<div bind:this={container} class="picker-inner"></div>
	</div>
</div>

<style>
	.picker-backdrop {
		position: fixed;
		inset: 0;
		z-index: 200;
		display: flex;
		align-items: flex-end;
		justify-content: flex-start;
		padding: 16px;
		padding-bottom: 80px;
	}

	.picker-container {
		background: #2b2d31;
		border-radius: 8px;
		overflow: hidden;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
		border: 1px solid #35373c;
		max-height: 400px;
	}

	.picker-inner {
		max-height: 400px;
		overflow-y: auto;
	}

	.picker-loading {
		padding: 20px;
		text-align: center;
		color: #b5bac1;
		font-size: 14px;
	}

	/* Override emoji-mart dark theme to match Discord palette */
	:global(em-emoji-picker) {
		--color-background: #2b2d31;
		--color-border: #35373c;
		--color-border-over: #4e505b;
		--color-header-background: #232428;
		--color-input: #1e1f22;
		--color-text: #dbdee1;
		--color-text-secondary: #b5bac1;
		--rgb-accent: 88, 101, 242;
	}

	:global(em-emoji-picker input) {
		background: #1e1f22 !important;
		border-color: #35373c !important;
		color: #dbdee1 !important;
	}

	:global(em-emoji-picker button:hover) {
		background: #35373c !important;
	}

	:global(em-emoji-picker .em-search) {
		padding: 8px !important;
	}
</style>
