<script lang="ts">
	/**
	 * @file Forum channel page — list of posts
	 * @purpose Shows forum posts with create new post form
	 */
	import { browser } from '$app/environment';
	import { forumStore } from '$lib/stores/forum.svelte';
	import { currentUser } from '$lib/stores/auth';

	let { params } = $props();
	let channelId = $state('');
	let serverId = $state('');
	let showNewPostForm = $state(false);
	let newPostTitle = $state('');
	let newPostContent = $state('');
	let creating = $state(false);
	let createError = $state('');

	if (browser) {
		channelId = params.channelId;
		serverId = params.serverId;
		if (channelId) {
			forumStore.fetchPosts(channelId).catch(() => {});
		}
	}

	async function handleCreatePost() {
		if (!newPostTitle.trim() || !newPostContent.trim()) return;
		creating = true;
		createError = '';
		try {
			await forumStore.createPost(channelId, newPostTitle.trim(), newPostContent.trim());
			newPostTitle = '';
			newPostContent = '';
			showNewPostForm = false;
		} catch (e: any) {
			createError = e.message || 'Failed to create post';
		} finally {
			creating = false;
		}
	}

	function goToPost(postId: string) {
		window.location.href = `/servers/${serverId}/forum/${channelId}/${postId}`;
	}

	function formatDate(dateStr: string): string {
		const date = new Date(dateStr);
		return date.toLocaleDateString() + ' ' + date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	}
</script>

<div class="flex-1 flex flex-col bg-gray-900">
	<!-- Header with title and new post button -->
	<div class="h-12 px-6 flex items-center justify-between border-b border-gray-800">
		<h1 class="text-lg font-bold text-white">Forum Posts</h1>
		<button
			onclick={() => { showNewPostForm = !showNewPostForm; createError = ''; }}
			class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white text-sm rounded font-medium transition-colors"
		>
			+ New Post
		</button>
	</div>

	<!-- New post form -->
	{#if showNewPostForm}
		<div class="px-6 py-4 bg-gray-850 border-b border-gray-800">
			{#if createError}
				<p class="text-red-400 text-sm mb-3">{createError}</p>
			{/if}
			<input
				type="text"
				bind:value={newPostTitle}
				placeholder="Post title"
				class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm placeholder-gray-500 focus:outline-none focus:border-indigo-500 mb-2"
			/>
			<textarea
				bind:value={newPostContent}
				placeholder="What's on your mind?"
				rows={4}
				class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm placeholder-gray-500 focus:outline-none focus:border-indigo-500 resize-none mb-3"
			></textarea>
			<div class="flex gap-2">
				<button
					onclick={handleCreatePost}
					disabled={!newPostTitle.trim() || !newPostContent.trim() || creating}
					class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white text-sm rounded font-medium transition-colors"
				>
					{creating ? 'Creating...' : 'Create Post'}
				</button>
				<button
					onclick={() => { showNewPostForm = false; createError = ''; }}
					class="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white text-sm rounded font-medium transition-colors"
				>
					Cancel
				</button>
			</div>
		</div>
	{/if}

	<!-- Posts list -->
	<div class="flex-1 overflow-y-auto">
		{#if forumStore.loading}
			<div class="flex items-center justify-center h-full">
				<p class="text-gray-400">Loading posts...</p>
			</div>
		{:else if forumStore.posts.length === 0}
			<div class="flex items-center justify-center h-full">
				<p class="text-gray-400">No posts yet. Be the first to post!</p>
			</div>
		{:else}
			<div class="space-y-2 p-4">
				{#each forumStore.posts as post (post.id)}
					<button
						onclick={() => goToPost(post.id)}
						class="w-full text-left p-4 bg-gray-800 hover:bg-gray-750 rounded border border-gray-700 hover:border-indigo-500 transition-all group"
					>
						<div class="flex items-start justify-between mb-1">
							<h3 class="font-semibold text-white group-hover:text-indigo-400 transition-colors">{post.title}</h3>
							{#if post.pinned}
								<span class="text-xs bg-yellow-600 text-white px-2 py-1 rounded">Pinned</span>
							{/if}
						</div>
						<p class="text-gray-300 text-sm mb-2 line-clamp-2">{post.content}</p>
						<div class="flex items-center justify-between text-xs text-gray-400">
							<span>{post.author_username} • {formatDate(post.created_at)}</span>
							<span class="text-indigo-400">{post.reply_count} {post.reply_count === 1 ? 'reply' : 'replies'}</span>
						</div>
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>
