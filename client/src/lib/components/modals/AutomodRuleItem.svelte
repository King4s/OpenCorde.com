<!--
  @component AutomodRuleItem
  @purpose Display and manage individual automod rules
  @uses Svelte 5 $props() rune
-->
<script lang="ts">
	interface Props {
		id: string;
		name: string;
		keywords: string[];
		action: string;
		enabled: boolean;
		onToggle: () => void;
		onDelete: () => void;
	}

	let { id, name, keywords, action, enabled, onToggle, onDelete }: Props = $props();
</script>

<div class="rule-item">
	<div class="rule-info">
		<strong>{name}</strong>
		<span class="keywords-preview"
			>{keywords.slice(0, 5).join(', ')}{keywords.length > 5 ? '...' : ''}</span
		>
		<span class="action-badge">{action}</span>
	</div>
	<div class="rule-actions">
		<button
			class="toggle"
			class:on={enabled}
			onclick={onToggle}
		>
			{enabled ? 'ON' : 'OFF'}
		</button>
		<button
			class="del-btn"
			onclick={onDelete}
		>
			🗑
		</button>
	</div>
</div>

<style>
	.rule-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		background: #1e1f22;
		border-radius: 6px;
		padding: 10px 12px;
	}

	.rule-info {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.rule-info strong {
		color: #f2f3f5;
		font-size: 13px;
	}

	.keywords-preview {
		color: #b5bac1;
		font-size: 12px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.action-badge {
		font-size: 10px;
		background: #35373c;
		color: #b5bac1;
		padding: 2px 6px;
		border-radius: 8px;
		width: fit-content;
	}

	.rule-actions {
		display: flex;
		gap: 6px;
		flex-shrink: 0;
		align-items: center;
	}

	.toggle {
		background: #35373c;
		border: none;
		border-radius: 4px;
		color: #b5bac1;
		padding: 4px 10px;
		cursor: pointer;
		font-size: 11px;
		font-weight: 700;
	}

	.toggle.on {
		background: #3ba55c;
		color: white;
	}

	.del-btn {
		background: none;
		border: none;
		cursor: pointer;
		padding: 4px;
		border-radius: 4px;
	}

	.del-btn:hover {
		background: #3c1618;
	}
</style>
