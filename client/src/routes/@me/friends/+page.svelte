<!--
  @page Friends
  @purpose View friends, manage pending requests, search users
-->
<script lang="ts">
	import { friendStore } from '$lib/stores/friends.svelte';
	import { currentUser } from '$lib/stores/auth';
	import { openDm } from '$lib/stores/dms';
	import { onMount } from 'svelte';
	import FriendItem from '$lib/components/layout/FriendItem.svelte';
	import AddFriendForm from '$lib/components/layout/AddFriendForm.svelte';

	let activeTab = $state<'all' | 'pending' | 'add'>('all');
	let addQuery = $state('');
	let addError = $state('');
	let addSuccess = $state('');

	onMount(async () => {
		await Promise.all([friendStore.fetchFriends(), friendStore.fetchPending()]);
	});

	async function handleSearch() {
		if (addQuery.length >= 2) await friendStore.search(addQuery);
	}

	async function handleSendRequest(userId: string) {
		addError = '';
		addSuccess = '';
		try {
			await friendStore.sendRequest(userId);
			addSuccess = 'Friend request sent!';
			friendStore.clearSearch();
			addQuery = '';
		} catch (e: unknown) {
			addError = (e as { message?: string }).message ?? 'Failed to send request';
		}
	}

	async function handleAccept(id: string) {
		await friendStore.accept(id);
	}

	async function handleRemove(id: string) {
		await friendStore.remove(id);
	}

	async function handleMessage(rel: { from_user: string; to_user: string }) {
		const me = $currentUser?.id;
		const otherUserId = rel.from_user === me ? rel.to_user : rel.from_user;
		if (!otherUserId) return;
		const dm = await openDm(otherUserId);
		window.location.href = `/@me/dms/${dm.id}`;
	}
</script>

<div class="friends-page">
	<div class="friends-header">
		<h1>Friends</h1>
		<div class="tabs">
			{#each [['all', 'All'], ['pending', 'Pending'], ['add', 'Add Friend']] as [id, label]}
				<button
					class="tab"
					class:active={activeTab === id}
					onclick={() => (activeTab = id as typeof activeTab)}
				>
					{label}
				</button>
			{/each}
		</div>
	</div>

	<div class="friends-content">
		{#if activeTab === 'all'}
			{#if friendStore.friends.length === 0}
				<div class="empty-state">
					<p>No friends yet. Add some!</p>
				</div>
			{:else}
				<div class="friend-list">
					{#each friendStore.friends as friend (friend.id)}
						<FriendItem
							id={friend.id}
							userId={friend.from_user === $currentUser?.id ? friend.to_user : friend.from_user}
							username={friend.other_username}
							avatarUrl={friend.other_avatar_url}
							type="friend"
							onMessage={() => handleMessage(friend)}
							onRemove={() => handleRemove(friend.id)}
						/>
					{/each}
				</div>
			{/if}
		{:else if activeTab === 'pending'}
			{#if friendStore.incoming.length > 0}
				<h3 class="section-title">Incoming — {friendStore.incoming.length}</h3>
				<div class="friend-list">
					{#each friendStore.incoming as req (req.id)}
						<FriendItem
							id={req.id}
							userId={req.from_user === $currentUser?.id ? req.to_user : req.from_user}
							username={req.other_username}
							avatarUrl={req.other_avatar_url}
							type="incoming"
							onAccept={() => handleAccept(req.id)}
							onRemove={() => handleRemove(req.id)}
						/>
					{/each}
				</div>
			{/if}
			{#if friendStore.outgoing.length > 0}
				<h3 class="section-title">Outgoing — {friendStore.outgoing.length}</h3>
				<div class="friend-list">
					{#each friendStore.outgoing as req (req.id)}
						<FriendItem
							id={req.id}
							userId={req.from_user === $currentUser?.id ? req.to_user : req.from_user}
							username={req.other_username}
							avatarUrl={req.other_avatar_url}
							type="outgoing"
							onRemove={() => handleRemove(req.id)}
						/>
					{/each}
				</div>
			{/if}
			{#if friendStore.incoming.length === 0 && friendStore.outgoing.length === 0}
				<div class="empty-state">
					<p>No pending requests.</p>
				</div>
			{/if}
		{:else if activeTab === 'add'}
			<AddFriendForm
				query={addQuery}
				error={addError}
				success={addSuccess}
				searchResults={friendStore.searchResults}
				onQueryChange={(q) => {
					addQuery = q;
					handleSearch();
				}}
				onSendRequest={handleSendRequest}
			/>
		{/if}
	</div>
</div>

<style>
	.friends-page {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: #313338;
	}
	.friends-header {
		padding: 16px 24px;
		border-bottom: 1px solid #1e1f22;
		display: flex;
		align-items: center;
		gap: 24px;
	}
	.friends-header h1 {
		margin: 0;
		font-size: 16px;
		font-weight: 700;
		color: #f2f3f5;
	}
	.tabs {
		display: flex;
		gap: 4px;
	}
	.tab {
		background: none;
		border: none;
		color: #b5bac1;
		padding: 6px 14px;
		border-radius: 4px;
		cursor: pointer;
		font-size: 14px;
	}
	.tab:hover {
		background: #35373c;
		color: #f2f3f5;
	}
	.tab.active {
		background: #35373c;
		color: #f2f3f5;
	}
	.friends-content {
		flex: 1;
		overflow-y: auto;
		padding: 24px;
	}
	.empty-state {
		text-align: center;
		color: #b5bac1;
		padding: 40px;
	}
	.section-title {
		color: #b5bac1;
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin: 16px 0 8px 0;
	}
	.friend-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
</style>
