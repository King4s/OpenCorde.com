<script lang="ts">
	/**
	 * @file Invite page — join a server via invite code
	 * @purpose Public invite landing, auth-gated join
	 */
	import { browser } from '$app/environment';
	import api from '$lib/api/client';

	let code = '';
	let serverName = $state('');
	let error = $state('');
	let loading = $state(true);
	let joined = $state(false);
	let needsLogin = $state(false);

	if (browser) {
		const match = window.location.pathname.match(/\/invite\/([^/]+)/);
		code = match?.[1] ?? '';
		loadInvite();
	}

	async function loadInvite() {
		if (!/^[A-Z0-9]{8}$/.test(code)) {
			error = 'Invalid or expired invite';
			loading = false;
			return;
		}

		try {
			const info = await api.get<{ server_name: string }>(`/invites/${code}`);
			serverName = info.server_name;
			loading = false;

			const token = localStorage.getItem('opencorde_token');
			if (!token) {
				needsLogin = true;
			}
		} catch {
			error = 'Invalid or expired invite';
			loading = false;
		}
	}

	async function handleJoin() {
		const token = localStorage.getItem('opencorde_token');
		if (!token) {
			localStorage.setItem('opencorde_pending_invite', code);
			window.location.href = '/login';
			return;
		}
		loading = true;
		try {
			await api.post(`/invites/${code}/join`);
			joined = true;
			loading = false;
			setTimeout(() => { window.location.href = '/servers'; }, 1500);
		} catch (e: any) {
			error = e.message || 'Failed to join';
			loading = false;
		}
	}
</script>

<div class="flex items-center justify-center min-h-screen bg-gray-900">
	<div class="w-full max-w-md p-8 bg-gray-800 rounded-lg shadow-xl text-center">
		{#if loading}
			<p class="text-gray-400">Loading invite...</p>
		{:else if error}
			<div class="text-6xl mb-4">&#x26A0;</div>
			<h1 class="text-xl font-bold text-white mb-2">Invalid Invite</h1>
			<p class="text-gray-400 mb-4">{error}</p>
			<a href="/" class="text-gray-400 hover:underline">Go home</a>
		{:else if joined}
			<div class="text-6xl mb-4">&#x2705;</div>
			<h1 class="text-xl font-bold text-white mb-2">Joined {serverName}!</h1>
			<p class="text-gray-400">Redirecting...</p>
		{:else}
			<div class="text-6xl mb-4">&#x1F517;</div>
			<h1 class="text-xl font-bold text-white mb-2">You've been invited to</h1>
			<h2 class="text-2xl font-bold text-gray-400 mb-6">{serverName}</h2>

			{#if needsLogin}
				<p class="text-gray-400 mb-4">You need an account to join</p>
				<div class="flex gap-3 justify-center">
					<a href="/login" class="px-6 py-2 bg-gray-600 hover:bg-gray-700 text-white rounded-lg">Log In</a>
					<a href="/register" class="px-6 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-lg">Register</a>
				</div>
			{:else}
				<button
					onclick={handleJoin}
					class="px-8 py-3 bg-gray-600 hover:bg-gray-700 text-white font-medium rounded-lg transition-colors"
				>
					Join Space
				</button>
			{/if}
		{/if}
	</div>
</div>
