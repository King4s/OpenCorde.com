<!--
  @component AttachmentPreview
  @purpose Display pending file attachments with remove buttons
  @version 1.0.0
-->
<script lang="ts">
	import type { Attachment } from '$lib/api/types';

	interface Props {
		attachments: Attachment[];
		onRemove: (id: string) => void;
	}

	let { attachments, onRemove }: Props = $props();

	function isImage(contentType: string): boolean {
		return contentType.startsWith('image/');
	}
</script>

{#if attachments.length > 0}
	<div class="preview-container">
		{#each attachments as att (att.id)}
			<div class="preview-item">
				{#if isImage(att.content_type)}
					<img src={att.url} alt={att.filename} class="preview-image" />
				{:else}
					<div class="preview-file">
						<span>📎</span>
						<span class="filename">{att.filename}</span>
					</div>
				{/if}
				<button
					onclick={() => onRemove(att.id)}
					class="remove-btn"
					aria-label="Remove attachment"
				>✕</button>
			</div>
		{/each}
	</div>
{/if}

<style>
	.preview-container {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		padding: 8px 12px;
		background: #2a2d31;
		border-radius: 6px;
		border: 1px solid #35373c;
	}

	.preview-item {
		position: relative;
		display: flex;
		align-items: center;
	}

	.preview-image {
		height: 64px;
		width: 64px;
		object-fit: cover;
		border-radius: 4px;
		border: 1px solid #35373c;
	}

	.preview-file {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 8px 12px;
		background: #1e1f22;
		border-radius: 4px;
		border: 1px solid #35373c;
		font-size: 12px;
		color: #b5bac1;
	}

	.filename {
		max-width: 120px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.remove-btn {
		position: absolute;
		top: -8px;
		right: -8px;
		width: 20px;
		height: 20px;
		padding: 0;
		border-radius: 50%;
		background: #ed4245;
		color: white;
		border: none;
		cursor: pointer;
		font-size: 12px;
		display: flex;
		align-items: center;
		justify-content: center;
		opacity: 0;
		transition: opacity 0.15s;
	}

	.preview-item:hover .remove-btn {
		opacity: 1;
	}

	.remove-btn:hover {
		background: #c41e3a;
	}
</style>
