<!--
  @component SlashCommandList
  @purpose Display list of registered slash commands with delete actions
  @version 1.0.0
-->
<script lang="ts">
	import type { SlashCommand } from '$lib/stores/slashCommands.svelte';

	interface Props {
		commands: SlashCommand[];
		onDelete: (id: string) => void;
	}

	let { commands, onDelete }: Props = $props();
</script>

<div class="commands-list">
	{#each commands as cmd (cmd.id)}
		<div class="command-item">
			<div class="command-info">
				<strong>/{cmd.name}</strong>
				{#if cmd.description}
					<span class="description">{cmd.description}</span>
				{/if}
				<span class="handler-preview">{cmd.handler_url}</span>
			</div>
			<div class="command-actions">
				<button
					class="del-btn"
					onclick={() => onDelete(cmd.id)}
					aria-label="Delete command"
				>
					🗑
				</button>
			</div>
		</div>
	{/each}
	{#if commands.length === 0}
		<p class="empty">No commands yet.</p>
	{/if}
</div>

<style>
	.commands-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.command-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		background: #1e1f22;
		border-radius: 6px;
		padding: 10px 12px;
		transition: background-color 0.15s;
	}

	.command-item:hover {
		background: #2a2d31;
	}

	.command-info {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
		flex: 1;
	}

	.command-info strong {
		color: #f2f3f5;
		font-size: 13px;
		font-family: monospace;
	}

	.description {
		color: #b5bac1;
		font-size: 12px;
	}

	.handler-preview {
		color: #949ba4;
		font-size: 11px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.command-actions {
		display: flex;
		gap: 6px;
		flex-shrink: 0;
	}

	.del-btn {
		background: none;
		border: none;
		cursor: pointer;
		padding: 4px;
		border-radius: 4px;
		font-size: 16px;
		transition: background-color 0.15s;
	}

	.del-btn:hover {
		background: #3c1618;
	}

	.empty {
		color: #b5bac1;
		font-size: 13px;
		text-align: center;
		padding: 12px;
		margin: 0;
	}
</style>
