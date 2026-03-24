<!--
  @component CommandAutocomplete
  @purpose Display and select from matching slash commands
  @version 1.0.0
-->
<script lang="ts">
	import type { SlashCommand } from '$lib/stores/slashCommands.svelte';

	interface Props {
		commands: SlashCommand[];
		onSelect: (name: string) => void;
	}

	let { commands, onSelect }: Props = $props();
</script>

<div class="command-autocomplete">
	{#each commands as cmd (cmd.id)}
		<button
			type="button"
			onclick={() => onSelect(cmd.name)}
			class="command-option"
		>
			<span class="command-name">/{cmd.name}</span>
			<span class="command-desc">{cmd.description || 'No description'}</span>
		</button>
	{/each}
</div>

<style>
	.command-autocomplete {
		position: absolute;
		bottom: 100%;
		left: 0;
		right: 0;
		background: #2a2d31;
		border: 1px solid #35373c;
		border-bottom: none;
		border-radius: 8px 8px 0 0;
		max-height: 200px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		z-index: 10;
	}

	.command-option {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px;
		border: none;
		background: none;
		color: #f2f3f5;
		cursor: pointer;
		text-align: left;
		border-bottom: 1px solid #35373c;
		transition: background-color 0.15s;
	}

	.command-option:hover {
		background: #35373c;
	}

	.command-option:last-child {
		border-bottom: none;
	}

	.command-name {
		font-family: monospace;
		font-size: 13px;
		font-weight: 600;
		color: #5865f2;
	}

	.command-desc {
		font-size: 12px;
		color: #949ba4;
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
