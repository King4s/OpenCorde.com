<!--
  @component KeyboardShortcutsHelp
  @purpose Shows keyboard shortcuts reference panel
  @version 1.0.0
-->
<script lang="ts">
	let { onClose }: { onClose: () => void } = $props();

	const shortcuts = [
		{ keys: ['Ctrl', 'K'], description: 'Quick search' },
		{ keys: ['Ctrl', ','], description: 'Open settings' },
		{ keys: ['Esc'], description: 'Close modal / cancel' },
		{ keys: ['Alt', 'Home'], description: 'Go to DMs' },
		{ keys: ['?'], description: 'Show this help' },
		{ keys: ['↑ / ↓'], description: 'Navigate messages (channel focused)' },
		{ keys: ['Enter'], description: 'Send message' },
		{ keys: ['Shift', 'Enter'], description: 'New line in message' }
	];
</script>

<div class="backdrop" onclick={onClose} role="presentation" tabindex="-1" onkeydown={(e) => e.key === 'Escape' && onClose()}>
	<div class="panel" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" tabindex="0" onkeydown={(e) => e.key === 'Escape' && onClose()}>
		<div class="panel-header">
			<h2>Keyboard Shortcuts</h2>
			<button onclick={onClose}>✕</button>
		</div>
		<div class="shortcuts-list">
			{#each shortcuts as s}
				<div class="shortcut-row">
					<div class="keys">
						{#each s.keys as key, i}
							<kbd>{key}</kbd>{#if i < s.keys.length - 1}<span class="plus">+</span>{/if}
						{/each}
					</div>
					<span class="desc">{s.description}</span>
				</div>
			{/each}
		</div>
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.7);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}
	.panel {
		background: #2b2d31;
		border-radius: 8px;
		width: 420px;
		max-width: 90vw;
	}
	.panel-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 16px 20px;
		border-bottom: 1px solid #35373c;
	}
	.panel-header h2 {
		margin: 0;
		font-size: 16px;
		color: #f2f3f5;
	}
	.panel-header button {
		background: none;
		border: none;
		color: #b5bac1;
		cursor: pointer;
		font-size: 18px;
	}
	.shortcuts-list {
		padding: 12px 20px 20px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.shortcut-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 6px 0;
	}
	.keys {
		display: flex;
		align-items: center;
		gap: 4px;
	}
	kbd {
		background: #1e1f22;
		border: 1px solid #4e505b;
		border-radius: 4px;
		padding: 3px 8px;
		font-size: 12px;
		color: #dbdee1;
		font-family: monospace;
	}
	.plus {
		color: #b5bac1;
		font-size: 11px;
	}
	.desc {
		font-size: 13px;
		color: #dbdee1;
	}
</style>
