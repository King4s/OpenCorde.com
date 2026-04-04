<script lang="ts">
	/**
	 * @file Server home page
	 * @purpose Landing page — create space, join via invite
	 * @depends stores/servers
	 */
	import { goto } from '$app/navigation';
	import { spaces, createSpace, selectSpace } from '$lib/stores/servers';
	import api from '$lib/api/client';

	let showCreateModal = $state(false);
	let showJoinModal = $state(false);
	let serverName = $state('');
	let serverDesc = $state('');
	let inviteCode = $state('');
	let error = $state('');
	let loading = $state(false);

	async function handleCreate() {
		if (!serverName.trim()) return;
		error = '';
		loading = true;
		try {
			const server = await createSpace(serverName.trim(), serverDesc.trim() || undefined);
			showCreateModal = false;
			serverName = '';
			serverDesc = '';
			window.location.href = '/servers';
		} catch (e: any) {
			error = e.message || 'Failed to create space';
		} finally {
			loading = false;
		}
	}

	async function handleJoin() {
		if (!inviteCode.trim()) return;
		error = '';
		loading = true;
		try {
			await api.post(`/invites/${inviteCode.trim()}/join`);
			showJoinModal = false;
			inviteCode = '';
			window.location.href = '/servers';
		} catch (e: any) {
			error = e.message || 'Invalid invite code';
		} finally {
			loading = false;
		}
	}

	function handleServerClick(id: string) {
		selectSpace(id);
		window.location.href = `/servers/${id}`;
	}
</script>

<div class="flex-1 flex flex-col items-center justify-center bg-gray-900 px-6">
	<div class="text-center max-w-md w-full">
		<div class="text-5xl mb-4" role="img" aria-label="Microphone">🎙️</div>
		<h1 class="text-3xl font-bold text-white mb-2">Welcome to OpenCorde</h1>

		{#if $spaces.length === 0}
			<p class="text-gray-400 mb-2">
				You haven't joined any spaces yet.
			</p>
			<p class="text-gray-500 text-sm mb-8">
				A <strong class="text-gray-400">space</strong> is a community or team hub — it contains channels, voice rooms, and members.
				Create your own or join one with an invite code.
			</p>
		{:else}
			<p class="text-gray-400 mb-8">
				Select a space from the sidebar to start chatting, or create a new one.
			</p>
		{/if}

		<div class="flex gap-4 justify-center">
			<button
				onclick={() => { showCreateModal = true; showJoinModal = false; error = ''; }}
				class="px-6 py-3 bg-indigo-600 hover:bg-indigo-500 text-white font-medium rounded-lg transition-colors"
			>
				Create a Space
			</button>
			<button
				onclick={() => { showJoinModal = true; showCreateModal = false; error = ''; }}
				class="px-6 py-3 bg-gray-700 hover:bg-gray-600 text-white font-medium rounded-lg transition-colors"
			>
				Join with Invite
			</button>
		</div>

		{#if showCreateModal}
			<div class="mt-6 w-full bg-gray-800 p-6 rounded-lg text-left">
				<h2 class="text-lg font-semibold text-white mb-1">Create a Space</h2>
				<p class="text-gray-500 text-sm mb-4">Give your community a name. You can add channels and invite people after creation.</p>
				{#if error}<p role="alert" class="text-red-400 text-sm mb-3">{error}</p>{/if}
				<div class="space-y-3">
					<div>
						<label for="space-name" class="block text-sm font-medium text-gray-300 mb-1">Space name <span class="text-gray-500 font-normal">(required)</span></label>
						<input
							id="space-name"
							type="text"
							bind:value={serverName}
							placeholder="e.g. My Team"
							class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500/50"
						/>
					</div>
					<div>
						<label for="space-desc" class="block text-sm font-medium text-gray-300 mb-1">Description <span class="text-gray-500 font-normal">(optional)</span></label>
						<input
							id="space-desc"
							type="text"
							bind:value={serverDesc}
							placeholder="What's this space about?"
							class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500/50"
						/>
					</div>
				</div>
				<div class="flex gap-2 mt-4">
					<button
						onclick={handleCreate}
						disabled={loading || !serverName.trim()}
						class="flex-1 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded transition-colors"
					>
						{loading ? 'Creating…' : 'Create Space'}
					</button>
					<button
						onclick={() => showCreateModal = false}
						class="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded transition-colors"
					>
						Cancel
					</button>
				</div>
			</div>
		{/if}

		{#if showJoinModal}
			<div class="mt-6 w-full bg-gray-800 p-6 rounded-lg text-left">
				<h2 class="text-lg font-semibold text-white mb-1">Join a Space</h2>
				<p class="text-gray-500 text-sm mb-4">Enter the invite code shared with you by a space admin or member.</p>
				{#if error}<p role="alert" class="text-red-400 text-sm mb-3">{error}</p>{/if}
				<div>
					<label for="invite-code" class="block text-sm font-medium text-gray-300 mb-1">Invite code</label>
					<input
						id="invite-code"
						type="text"
						bind:value={inviteCode}
						placeholder="Paste invite code here"
						class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500/50 mb-4"
					/>
				</div>
				<div class="flex gap-2">
					<button
						onclick={handleJoin}
						disabled={loading || !inviteCode.trim()}
						class="flex-1 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded transition-colors"
					>
						{loading ? 'Joining…' : 'Join Space'}
					</button>
					<button
						onclick={() => showJoinModal = false}
						class="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded transition-colors"
					>
						Cancel
					</button>
				</div>
			</div>
		{/if}

		{#if $spaces.length > 0}
			<div class="mt-8 text-left w-full">
				<p class="text-gray-400 text-sm mb-2 font-medium">Your spaces</p>
				{#each $spaces as server (server.id)}
					<button
						onclick={() => handleServerClick(server.id)}
						class="block w-full text-left px-4 py-2.5 rounded-lg hover:bg-gray-800 text-gray-300 hover:text-white text-sm transition-colors"
					>
						{server.name}
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>
