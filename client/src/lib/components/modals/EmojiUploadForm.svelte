<!--
  @component EmojiUploadForm
  @purpose Form to upload a new custom emoji
  @version 1.0.0
-->
<script lang="ts">
	interface Props {
		name: string;
		file: File | null;
		error: string;
		isLoading: boolean;
		onNameChange: (value: string) => void;
		onFileChange: (file: File | null) => void;
		onSubmit: () => void;
	}

	let {
		name,
		file,
		error,
		isLoading,
		onNameChange,
		onFileChange,
		onSubmit
	}: Props = $props();

	function handleFileInput(e: Event) {
		const input = e.target as HTMLInputElement;
		onFileChange(input.files?.[0] ?? null);
	}
</script>

<div class="form">
	<h3>Upload Emoji</h3>
	<label>
		Name
		<input
			type="text"
			value={name}
			onchange={(e) => onNameChange((e.target as HTMLInputElement).value)}
			placeholder="e.g., happy"
			disabled={isLoading}
		/>
	</label>
	<label>
		File (PNG, GIF, WebP, max 256KB)
		<input
			type="file"
			onchange={handleFileInput}
			accept=".png,.gif,.webp,image/png,image/gif,image/webp"
			disabled={isLoading}
		/>
	</label>
	{#if error}
		<p class="err">{error}</p>
	{/if}
	<button onclick={onSubmit} disabled={isLoading || !name.trim() || !file}>
		{isLoading ? 'Uploading…' : 'Upload'}
	</button>
</div>

<style>
	.form {
		border-top: 1px solid #35373c;
		padding-top: 16px;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.form h3 {
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
		border-color: #e5e7eb;
	}

	input:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.err {
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
