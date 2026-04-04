<!--
  @component WebhookManager
  @purpose Modal for creating and managing incoming webhooks for a channel.
  @version 1.0.0
-->
<script lang="ts">
	import { webhookStore } from '$lib/stores/webhooks.svelte';
	import { onMount } from 'svelte';
	import { edgeResize } from '$lib/actions/edgeResize';

	let { channelId, onClose }: { channelId: string; onClose: () => void } = $props();

	let newName = $state('');
	let creating = $state(false);
	let copiedId = $state<string | null>(null);

	onMount(() => {
		webhookStore.fetchForChannel(channelId);
	});

	async function handleCreate() {
		if (!newName.trim()) return;
		creating = true;
		try {
			await webhookStore.create(channelId, newName.trim());
			newName = '';
		} finally {
			creating = false;
		}
	}

	async function handleDelete(id: string) {
		if (!confirm('Delete this webhook?')) return;
		await webhookStore.remove(id);
	}

	function copyUrl(webhook: { url: string; id: string }) {
		const fullUrl = `${window.location.origin}${webhook.url}`;
		navigator.clipboard.writeText(fullUrl);
		copiedId = webhook.id;
		setTimeout(() => {
			copiedId = null;
		}, 2000);
	}
</script>

<div class="modal-backdrop" onclick={onClose} role="presentation" onkeydown={(e) => e.key === 'Escape' && onClose()}>
	<div use:edgeResize={{ handles: ['left', 'right'], minWidth: 420, maxWidth: 920 }} class="modal resizable-modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" tabindex="-1" onkeydown={(e) => e.key === 'Escape' && onClose()}>
		<div class="modal-header">
			<h2>Webhooks</h2>
			<button class="close-btn" onclick={onClose}>✕</button>
		</div>

		<div class="modal-body">
			<div class="create-form">
				<input
					bind:value={newName}
					placeholder="Webhook name"
					onkeydown={(e) => e.key === 'Enter' && handleCreate()}
				/>
				<button onclick={handleCreate} disabled={creating || !newName.trim()}>
					{creating ? '...' : 'Create'}
				</button>
			</div>

			{#if webhookStore.loading}
				<p class="loading">Loading webhooks...</p>
			{:else if webhookStore.webhooks.length === 0}
				<p class="empty">No webhooks yet. Create one above.</p>
			{:else}
				<ul class="webhook-list">
					{#each webhookStore.webhooks as wh (wh.id)}
						<li class="webhook-item">
							<div class="webhook-info">
								<span class="webhook-name">{wh.name}</span>
								<code class="webhook-url">{wh.url}</code>
							</div>
							<div class="webhook-actions">
								<button
									class="copy-btn"
									onclick={() => copyUrl(wh)}
									title="Copy URL"
								>
									{copiedId === wh.id ? '✓' : '📋'}
								</button>
								<button
									class="delete-btn"
									onclick={() => handleDelete(wh.id)}
									title="Delete webhook"
								>
									🗑
								</button>
							</div>
						</li>
					{/each}
				</ul>
			{/if}

			<div class="help-text">
				<p>POST JSON to the webhook URL to send a message:</p>
				<code>{`{ "content": "Hello!", "username": "MyBot" }`}</code>
			</div>
		</div>
	</div>
</div>

<style>
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.7);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}
	.modal {
		background: #2b2d31;
		border-radius: 8px;
		width: 480px;
		max-width: 90vw;
		max-height: 80vh;
		display: flex;
		flex-direction: column;
	}

	.resizable-modal {
		overflow: auto;
		min-width: 420px;
		min-height: 360px;
		max-width: min(90vw, 920px);
		max-height: min(85vh, 780px);
	}
	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 16px 20px;
		border-bottom: 1px solid #1e1f22;
	}
	.modal-header h2 {
		margin: 0;
		font-size: 16px;
		color: #f2f3f5;
	}
	.close-btn {
		background: none;
		border: none;
		color: #b5bac1;
		cursor: pointer;
		font-size: 18px;
		padding: 4px 8px;
		border-radius: 4px;
	}
	.close-btn:hover {
		background: #35373c;
		color: #f2f3f5;
	}
	.modal-body {
		padding: 16px 20px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 16px;
	}
	.create-form {
		display: flex;
		gap: 8px;
	}
	.create-form input {
		flex: 1;
		background: #1e1f22;
		border: 1px solid #35373c;
		border-radius: 4px;
		color: #dbdee1;
		padding: 8px 12px;
		font-size: 14px;
		outline: none;
	}
	.create-form input:focus {
		border-color: #e5e7eb;
	}
	.create-form button {
		background: #e5e7eb;
		border: none;
		border-radius: 4px;
		color: white;
		padding: 8px 16px;
		cursor: pointer;
		font-size: 14px;
		white-space: nowrap;
	}
	.create-form button:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.loading,
	.empty {
		color: #b5bac1;
		font-size: 13px;
	}
	.webhook-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.webhook-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		background: #1e1f22;
		border-radius: 6px;
		padding: 10px 12px;
		gap: 8px;
	}
	.webhook-info {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}
	.webhook-name {
		font-size: 13px;
		font-weight: 600;
		color: #f2f3f5;
	}
	.webhook-url {
		font-size: 11px;
		color: #b5bac1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		max-width: 300px;
	}
	.webhook-actions {
		display: flex;
		gap: 4px;
		flex-shrink: 0;
	}
	.copy-btn,
	.delete-btn {
		background: none;
		border: none;
		cursor: pointer;
		padding: 4px 8px;
		border-radius: 4px;
		font-size: 14px;
	}
	.copy-btn:hover {
		background: #35373c;
	}
	.delete-btn:hover {
		background: #3c1618;
	}
	.help-text {
		border-top: 1px solid #35373c;
		padding-top: 12px;
	}
	.help-text p {
		font-size: 12px;
		color: #b5bac1;
		margin: 0 0 6px 0;
	}
	.help-text code {
		font-size: 11px;
		color: #dbdee1;
		background: #1e1f22;
		padding: 6px 10px;
		border-radius: 4px;
		display: block;
	}
</style>
