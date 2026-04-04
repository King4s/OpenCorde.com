<!--
  @component FriendItem
  @purpose Displays a single friend or friend request with action buttons
  @uses Svelte 5 $props() rune
-->
<script lang="ts">
	import { presenceMap } from '$lib/stores/presence';

	interface Props {
		id: string;
		userId: string;
		username: string;
		avatarUrl: string | null;
		type: 'friend' | 'incoming' | 'outgoing';
		onAccept?: () => void;
		onRemove: () => void;
		onMessage?: () => void;
	}

	let { id, userId, username, avatarUrl, type, onAccept, onRemove, onMessage }: Props = $props();

	function getInitials(name: string) {
		return name.slice(0, 2).toUpperCase();
	}
</script>

<div class="friend-item">
	<div class="friend-avatar">
		{#if avatarUrl}
			<img src={avatarUrl} alt={username} />
		{:else}
			<span class="initials">{getInitials(username)}</span>
		{/if}
		{#if $presenceMap.has(userId)}
			<span class="presence-dot online"></span>
		{/if}
	</div>
	<div class="friend-main">
		<span class="friend-name">{username}</span>
		{#if type === 'friend'}
			<span class="friend-status">{$presenceMap.has(userId) ? 'Online' : 'Offline'}</span>
		{:else if type === 'incoming'}
			<span class="friend-status">Incoming request</span>
		{:else}
			<span class="friend-status">Outgoing request</span>
		{/if}
	</div>
	<div class="friend-actions">
		{#if type === 'friend' && onMessage}
			<button
				class="action-btn message"
				onclick={onMessage}
				title="Message"
			>
				💬
			</button>
		{/if}
		{#if type === 'incoming'}
			<button
				class="action-btn accept"
				onclick={onAccept}
				title="Accept"
			>
				✓
			</button>
		{/if}
		<button
			class="action-btn danger"
			onclick={onRemove}
			title={type === 'friend' ? 'Remove friend' : type === 'incoming' ? 'Decline' : 'Cancel'}
		>
			✕
		</button>
	</div>
</div>

<style>
	.friend-item {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 10px 12px;
		border-radius: 8px;
		cursor: pointer;
	}
	.friend-item:hover {
		background: #35373c;
	}
	.friend-avatar {
		width: 36px;
		height: 36px;
		border-radius: 50%;
		background: #e5e7eb;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		overflow: hidden;
		position: relative;
	}
	.friend-avatar img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.initials {
		font-size: 12px;
		font-weight: 700;
		color: white;
	}
	.presence-dot {
		position: absolute;
		right: -1px;
		bottom: -1px;
		width: 11px;
		height: 11px;
		border-radius: 999px;
		border: 2px solid #313338;
		background: #6b7280;
	}
	.presence-dot.online {
		background: #3ba55c;
	}
	.friend-main {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.friend-name {
		font-size: 15px;
		font-weight: 600;
		color: #f2f3f5;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.friend-status {
		font-size: 12px;
		color: #8b949e;
	}
	.friend-actions {
		display: flex;
		gap: 6px;
		opacity: 0;
	}
	.friend-item:hover .friend-actions {
		opacity: 1;
	}
	.action-btn {
		background: #35373c;
		border: none;
		border-radius: 50%;
		width: 32px;
		height: 32px;
		cursor: pointer;
		font-size: 14px;
		color: #b5bac1;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.action-btn.message {
		color: #e5e7eb;
	}
	.action-btn.message:hover {
		background: #e5e7eb;
		color: white;
	}
	.action-btn.accept {
		color: #3ba55c;
	}
	.action-btn.accept:hover {
		background: #3ba55c;
		color: white;
	}
	.action-btn.danger:hover {
		background: #ed4245;
		color: white;
	}
</style>
