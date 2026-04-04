<!--
  @component AutomodRuleForm
  @purpose Form for creating new automod rules
  @uses Svelte 5 $props() rune
-->
<script lang="ts">
	interface Props {
		name: string;
		keywords: string;
		action: string;
		error: string;
		creating: boolean;
		onNameChange: (name: string) => void;
		onKeywordsChange: (keywords: string) => void;
		onActionChange: (action: string) => void;
		onSubmit: () => void;
	}

	let { name, keywords, action, error, creating, onNameChange, onKeywordsChange, onActionChange, onSubmit }: Props = $props();
</script>

<div class="add-rule">
	<h3>Add Rule</h3>
	<label>
		Rule Name
		<input
			value={name}
			oninput={(e) => onNameChange((e.target as HTMLInputElement).value)}
			placeholder="Spam Filter"
		/>
	</label>
	<label>
		Keywords (one per line)
		<textarea
			value={keywords}
			oninput={(e) => onKeywordsChange((e.target as HTMLTextAreaElement).value)}
			placeholder="badword
spam phrase
offensive term"
			rows="4"
		></textarea>
	</label>
	<label>
		Action
		<select value={action} onchange={(e) => onActionChange((e.target as HTMLSelectElement).value)}>
			<option value="delete">Delete message</option>
			<option value="timeout">Timeout user (future)</option>
		</select>
	</label>
	{#if error}
		<p class="error">{error}</p>
	{/if}
	<button
		onclick={onSubmit}
		disabled={creating}
	>
		{creating ? 'Creating...' : 'Create Rule'}
	</button>
</div>

<style>
	.add-rule {
		border-top: 1px solid #35373c;
		padding-top: 16px;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.add-rule h3 {
		margin: 0;
		font-size: 14px;
		color: #f2f3f5;
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 4px;
		font-size: 11px;
		color: #b5bac1;
		font-weight: 700;
		text-transform: uppercase;
	}

	input,
	textarea,
	select {
		background: #1e1f22;
		border: 1px solid #35373c;
		border-radius: 4px;
		color: #dbdee1;
		padding: 8px 12px;
		font-size: 14px;
		outline: none;
		font-family: inherit;
	}

	input:focus,
	textarea:focus,
	select:focus {
		border-color: #e5e7eb;
	}

	textarea {
		resize: vertical;
	}

	.error {
		color: #ed4245;
		font-size: 13px;
		margin: 0;
	}

	button {
		background: #e5e7eb;
		border: none;
		border-radius: 4px;
		color: white;
		padding: 8px 20px;
		cursor: pointer;
		font-size: 14px;
		align-self: flex-start;
	}

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
