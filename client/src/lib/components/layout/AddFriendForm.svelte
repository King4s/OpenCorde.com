<!--
  @component AddFriendForm
  @purpose Search for and add new friends
  @uses Svelte 5 $props() rune
-->
<script lang="ts">
	interface SearchResult {
		id: string;
		username: string;
	}

	interface Props {
		query: string;
		error: string;
		success: string;
		searchResults: SearchResult[];
		onQueryChange: (query: string) => void;
		onSendRequest: (userId: string) => void;
	}

	let { query, error, success, searchResults, onQueryChange, onSendRequest }: Props = $props();
</script>

<div class="add-friend">
	<h3>Add Friend</h3>
	<p class="hint">Search by username to send a friend request.</p>
	<div class="search-row">
		<input
			value={query}
			placeholder="Search username..."
			oninput={(e) => onQueryChange((e.target as HTMLInputElement).value)}
		/>
	</div>
	{#if error}
		<p class="error">{error}</p>
	{/if}
	{#if success}
		<p class="success">{success}</p>
	{/if}
	{#if searchResults.length > 0}
		<div class="search-results">
			{#each searchResults as user (user.id)}
				<div class="result-item">
					<span class="result-name">{user.username}</span>
					<button
						class="action-btn accept"
						onclick={() => onSendRequest(user.id)}
					>
						Add
					</button>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.add-friend {
		max-width: 500px;
	}
	.add-friend h3 {
		color: #f2f3f5;
		margin: 0 0 8px 0;
	}
	.hint {
		color: #b5bac1;
		font-size: 14px;
		margin: 0 0 16px 0;
	}
	.search-row {
		display: flex;
		gap: 8px;
	}
	.search-row input {
		flex: 1;
		background: #1e1f22;
		border: 1px solid #35373c;
		border-radius: 4px;
		color: #dbdee1;
		padding: 10px 14px;
		font-size: 15px;
		outline: none;
	}
	.search-row input:focus {
		border-color: #e5e7eb;
	}
	.error {
		color: #ed4245;
		font-size: 13px;
		margin: 8px 0 0 0;
	}
	.success {
		color: #3ba55c;
		font-size: 13px;
		margin: 8px 0 0 0;
	}
	.search-results {
		margin-top: 16px;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}
	.result-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 10px 12px;
		background: #2b2d31;
		border-radius: 6px;
	}
	.result-name {
		font-size: 14px;
		color: #f2f3f5;
		font-weight: 600;
	}
	.result-item .action-btn.accept {
		width: auto;
		border-radius: 4px;
		padding: 6px 16px;
		background: #e5e7eb;
		color: white;
		font-size: 13px;
	}
	.result-item .action-btn.accept:hover {
		background: #4752c4;
	}
	.action-btn {
		background: #35373c;
		border: none;
		border-radius: 50%;
		cursor: pointer;
		font-size: 14px;
		color: #b5bac1;
		display: flex;
		align-items: center;
		justify-content: center;
	}
</style>
