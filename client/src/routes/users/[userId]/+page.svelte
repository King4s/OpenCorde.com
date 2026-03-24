<script lang="ts">
	/**
	 * @file User profile page
	 * @purpose Show public profile for a user
	 */
	import { browser } from '$app/environment';
	import api from '$lib/api/client';

	interface PublicProfile {
		id: string;
		username: string;
		avatar_url: string | null;
		status: number;
	}

	let userId = $state('');
	let profile = $state<PublicProfile | null>(null);
	let loading = $state(true);
	let error = $state('');

	if (browser) {
		const match = window.location.pathname.match(/\/users\/([^/]+)/);
		const id = match?.[1] ?? '';
		userId = id;
		if (id) {
			api.get<PublicProfile>(`/users/${id}`)
				.then(p => { profile = p; loading = false; })
				.catch(e => { error = e.message ?? 'Failed to load profile'; loading = false; });
		}
	}

	function getInitials(name: string): string {
		return name.slice(0, 2).toUpperCase();
	}

	function getAvatarColor(id: string): string {
		const colors = ['bg-indigo-600', 'bg-purple-600', 'bg-pink-600', 'bg-red-600', 'bg-orange-600', 'bg-teal-600'];
		const hash = id.split('').reduce((a, c) => a + c.charCodeAt(0), 0);
		return colors[hash % colors.length];
	}

	const statusLabel: Record<number, string> = { 0: 'Offline', 1: 'Online', 2: 'Idle', 3: 'Do Not Disturb' };
</script>

<div class="min-h-screen bg-gray-900 flex items-center justify-center p-8">
	{#if loading}
		<p class="text-gray-400">Loading…</p>
	{:else if error}
		<p class="text-red-400">{error}</p>
	{:else if profile}
		<div class="bg-gray-800 rounded-xl shadow-xl w-full max-w-sm overflow-hidden">
			<!-- Banner -->
			<div class="h-24 bg-gradient-to-r from-indigo-800 to-purple-800"></div>
			<!-- Avatar -->
			<div class="px-6 pb-6">
				<div class="-mt-10 mb-4">
					{#if profile.avatar_url}
						<img src={profile.avatar_url} alt={profile.username} class="w-20 h-20 rounded-full border-4 border-gray-800 object-cover" />
					{:else}
						<div class="w-20 h-20 rounded-full border-4 border-gray-800 {getAvatarColor(profile.id)} flex items-center justify-center text-white text-2xl font-bold">
							{getInitials(profile.username)}
						</div>
					{/if}
				</div>
				<h1 class="text-xl font-bold text-white">{profile.username}</h1>
				<p class="text-gray-400 text-sm mt-0.5">{statusLabel[profile.status] ?? 'Unknown'}</p>
				<div class="mt-4 pt-4 border-t border-gray-700">
					<p class="text-xs text-gray-500">ID: {profile.id}</p>
				</div>
			</div>
		</div>
	{/if}
</div>
