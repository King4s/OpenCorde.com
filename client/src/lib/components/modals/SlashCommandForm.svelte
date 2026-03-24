<!--
  @component SlashCommandForm
  @purpose Form to register a new slash command
  @version 1.0.0
-->
<script lang="ts">
	interface Props {
		onSubmit: () => void;
		onNameChange: (value: string) => void;
		onDescriptionChange: (value: string) => void;
		onHandlerUrlChange: (value: string) => void;
		name: string;
		description: string;
		handlerUrl: string;
		error: string;
		isLoading: boolean;
	}

	let {
		onSubmit,
		onNameChange,
		onDescriptionChange,
		onHandlerUrlChange,
		name,
		description,
		handlerUrl,
		error,
		isLoading
	}: Props = $props();
</script>

<div class="add-command">
	<h3>Register Command</h3>
	<label>
		Command Name
		<input
			value={name}
			onchange={(e) => onNameChange((e.target as HTMLInputElement).value)}
			placeholder="ping"
			maxlength="32"
			disabled={isLoading}
		/>
	</label>
	<label>
		Description (optional)
		<input
			value={description}
			onchange={(e) => onDescriptionChange((e.target as HTMLInputElement).value)}
			placeholder="Responds with pong"
			maxlength="100"
			disabled={isLoading}
		/>
	</label>
	<label>
		Handler URL
		<input
			value={handlerUrl}
			onchange={(e) => onHandlerUrlChange((e.target as HTMLInputElement).value)}
			placeholder="https://example.com/api/commands/ping"
			disabled={isLoading}
		/>
	</label>
	{#if error}
		<p class="error">{error}</p>
	{/if}
	<button onclick={onSubmit} disabled={isLoading}>
		{isLoading ? 'Registering...' : 'Register Command'}
	</button>
</div>

<style>
	.add-command {
		border-top: 1px solid #35373c;
		padding-top: 16px;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.add-command h3 {
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

	input {
		background: #1e1f22;
		border: 1px solid #35373c;
		border-radius: 4px;
		color: #dbdee1;
		padding: 8px 12px;
		font-size: 14px;
		outline: none;
		font-family: inherit;
	}

	input:focus {
		border-color: #5865f2;
	}

	input:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.error {
		color: #ed4245;
		font-size: 13px;
		margin: 0;
	}

	button {
		background: #5865f2;
		border: none;
		border-radius: 4px;
		color: white;
		padding: 8px 20px;
		cursor: pointer;
		font-size: 14px;
		align-self: flex-start;
		transition: background-color 0.15s;
	}

	button:hover:not(:disabled) {
		background: #4752c4;
	}

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
