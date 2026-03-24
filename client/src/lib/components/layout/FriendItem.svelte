<!--
  @component FriendItem
  @purpose Displays a single friend or friend request with action buttons
  @uses Svelte 5 $props() rune
-->
<script lang="ts">
	interface Props {
		id: string;
		username: string;
		avatarUrl: string | null;
		type: 'friend' | 'incoming' | 'outgoing';
		onAccept?: () => void;
		onRemove: () => void;
	}

	let { id, username, avatarUrl, type, onAccept, onRemove }: Props = $props();

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
	</div>
	<span class="friend-name">{username}</span>
	<div class="friend-actions">
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
		background: #5865f2;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		overflow: hidden;
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
	.friend-name {
		flex: 1;
		font-size: 15px;
		font-weight: 600;
		color: #f2f3f5;
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
