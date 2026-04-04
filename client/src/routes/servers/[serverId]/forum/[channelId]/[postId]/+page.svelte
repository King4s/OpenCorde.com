<script lang="ts">
	/**
	 * @file Post detail page — single post with replies
	 * @purpose Shows post content, list of replies, and reply form
	 */
	import { browser } from '$app/environment';
	import { forumStore } from '$lib/stores/forum.svelte';
	import { currentUser } from '$lib/stores/auth';

	let { params } = $props();
	let postId = $state('');
	let channelId = $state('');
	let spaceId = $state('');
	let replyContent = $state('');
	let replying = $state(false);
	let replyError = $state('');

	if (browser) {
		const match = window.location.pathname.match(/^\/servers\/([^/]+)\/forum\/([^/]+)\/([^/]+)/);
		const sid = match?.[1] ?? '';
		const cid = match?.[2] ?? '';
		const pid = match?.[3] ?? '';
		postId = pid;
		channelId = cid;
		spaceId = sid;
		if (pid) {
			forumStore.fetchPost(pid).catch(() => {});
		}
	}

	async function handleCreateReply() {
		if (!replyContent.trim()) return;
		replying = true;
		replyError = '';
		try {
			await forumStore.createReply(postId, replyContent.trim());
			replyContent = '';
		} catch (e: any) {
			replyError = e.message || 'Failed to create reply';
		} finally {
			replying = false;
		}
	}

	async function handleDeletePost() {
		if (!forumStore.currentPost) return;
		if (!confirm(`Delete this post? This cannot be undone.`)) return;
		try {
			await forumStore.deletePost(postId);
			window.location.href = `/servers/${spaceId}/forum/${channelId}`;
		} catch (e: any) {
			alert(e.message || 'Failed to delete post');
		}
	}

	async function handleDeleteReply(replyId: string) {
		if (!confirm('Delete this reply? This cannot be undone.')) return;
		try {
			await forumStore.deleteReply(replyId);
		} catch (e: any) {
			alert(e.message || 'Failed to delete reply');
		}
	}

	function formatDate(dateStr: string): string {
		const date = new Date(dateStr);
		return date.toLocaleDateString() + ' ' + date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	}

	function canDeletePost(): boolean {
		if (!forumStore.currentPost) return false;
		return forumStore.currentPost.author_id === $currentUser?.id;
	}

	function canDeleteReply(authorId: string): boolean {
		return authorId === $currentUser?.id;
	}
</script>

<div class="flex-1 flex flex-col bg-gray-900">
	<!-- Header with back button -->
	<div class="h-12 px-6 flex items-center border-b border-gray-800">
		<a
			href={`/servers/${spaceId}/forum/${channelId}`}
			class="text-gray-400 hover:text-gray-300 text-sm font-medium"
		>
			← Back to Posts
		</a>
	</div>

	<!-- Post content -->
	{#if forumStore.loading}
		<div class="flex-1 flex items-center justify-center">
			<p class="text-gray-400">Loading post...</p>
		</div>
	{:else if !forumStore.currentPost}
		<div class="flex-1 flex items-center justify-center">
			<p class="text-gray-400">Post not found</p>
		</div>
	{:else}
		<div class="flex-1 overflow-y-auto">
			<!-- Post -->
			<div class="max-w-4xl mx-auto w-full p-6 border-b border-gray-800">
				<div class="flex items-start justify-between mb-2">
					<h1 class="text-2xl font-bold text-white">{forumStore.currentPost.title}</h1>
					{#if canDeletePost()}
						<button
							onclick={handleDeletePost}
							class="px-3 py-1 text-xs text-gray-400 hover:text-gray-300 hover:bg-gray-900/20 rounded transition-colors"
						>
							Delete
						</button>
					{/if}
				</div>
				<p class="text-gray-400 text-sm mb-4">
					by <span class="text-gray-300 font-medium">{forumStore.currentPost.author_username}</span>
					on {formatDate(forumStore.currentPost.created_at)}
				</p>
				<div class="text-gray-200 whitespace-pre-wrap mb-4">
					{forumStore.currentPost.content}
				</div>
				<div class="flex gap-2 text-xs text-gray-400">
					<span>{forumStore.currentPost.reply_count} {forumStore.currentPost.reply_count === 1 ? 'reply' : 'replies'}</span>
				</div>
			</div>

			<!-- Replies -->
			{#if forumStore.replies.length > 0}
				<div class="max-w-4xl mx-auto w-full p-6 space-y-4">
					{#each forumStore.replies as reply (reply.id)}
						<div class="p-4 bg-gray-800 rounded border border-gray-700">
							<div class="flex items-start justify-between mb-2">
								<span class="text-sm font-medium text-gray-300">{reply.author_username}</span>
								{#if canDeleteReply(reply.author_id)}
									<button
										onclick={() => handleDeleteReply(reply.id)}
										class="text-xs text-gray-400 hover:text-gray-300 hover:bg-gray-900/20 px-2 py-1 rounded transition-colors"
									>
										Delete
									</button>
								{/if}
							</div>
							<p class="text-xs text-gray-400 mb-2">{formatDate(reply.created_at)}</p>
							<p class="text-gray-200 text-sm whitespace-pre-wrap">{reply.content}</p>
						</div>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Reply form -->
		<div class="max-w-4xl mx-auto w-full px-6 py-4 border-t border-gray-800 bg-gray-850">
			{#if replyError}
				<p class="text-gray-400 text-sm mb-2">{replyError}</p>
			{/if}
			<textarea
				bind:value={replyContent}
				placeholder="Write a reply..."
				rows={3}
				class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm placeholder-gray-500 focus:outline-none focus:border-gray-500 resize-none mb-2"
			></textarea>
			<button
				onclick={handleCreateReply}
				disabled={!replyContent.trim() || replying}
				class="px-4 py-2 bg-gray-600 hover:bg-gray-700 disabled:opacity-50 text-white text-sm rounded font-medium transition-colors"
			>
				{replying ? 'Posting...' : 'Post Reply'}
			</button>
		</div>
	{/if}
</div>
