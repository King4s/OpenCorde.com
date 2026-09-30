<script lang="ts">
	/**
	 * @file SessionsPanel — active session/device management panel
	 * @purpose List active sessions and remotely log out
	 * @depends api/client, $lib/stores/toasts.svelte
	 */
	import api from '$lib/api/client';
	import { toastError, toastSuccess } from '$lib/stores/toasts.svelte';

	interface Session {
		id: string;
		created_at: string;
		expires_at: string;
		current: boolean;
	}

	let sessions: Session[] = $state([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	async function loadSessions() {
		loading = true;
		error = null;
		try {
			sessions = await api.get<Session[]>('/users/@me/sessions');
		} catch (e) {
			error = 'Failed to load sessions';
			console.error('[sessions]', e);
		} finally {
			loading = false;
		}
	}

	async function revokeSession(id: string) {
		try {
			await api.delete(`/users/@me/sessions/${id}`);
			sessions = sessions.filter((s) => s.id !== id);
			toastSuccess('Session logged out');
		} catch (e) {
			toastError('Failed to revoke session');
			console.error('[sessions] revoke:', e);
		}
	}

	async function revokeAllOthers() {
		if (!confirm('Log out all other sessions?')) return;
		try {
			await api.delete('/users/@me/sessions');
			sessions = sessions.filter((s) => s.current);
			toastSuccess('All other sessions logged out');
		} catch (e) {
			toastError('Failed to revoke sessions');
			console.error('[sessions] revoke all:', e);
		}
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleString();
	}

	function timeAgo(iso: string): string {
		const diff = Date.now() - new Date(iso).getTime();
		const mins = Math.floor(diff / 60000);
		if (mins < 1) return 'just now';
		if (mins < 60) return `${mins}m ago`;
		const hours = Math.floor(mins / 60);
		if (hours < 24) return `${hours}h ago`;
		return `${Math.floor(hours / 24)}d ago`;
	}

	$effect(() => {
		loadSessions();
	});
</script>

<div class="bg-gray-800 rounded-lg p-3 sm:p-4">
	<h2 class="text-sm font-semibold text-gray-400 uppercase mb-3">Sessions &amp; Devices</h2>

	{#if loading}
		<p class="text-xs text-gray-500">Loading sessions...</p>
	{:else if error}
		<p class="text-xs text-red-400">{error}</p>
		<button onclick={loadSessions} class="text-xs text-blue-400 hover:underline mt-1">Retry</button>
	{:else if sessions.length === 0}
		<p class="text-xs text-gray-500">No active sessions.</p>
	{/if}

	{#if sessions.length > 0}
		<div class="space-y-2 mt-2">
			{#each sessions as session (session.id)}
				<div class="flex items-center justify-between bg-gray-750 rounded p-2 text-xs">
					<div class="flex-1 min-w-0">
						<div class="text-gray-300 truncate">
							{session.current ? '🖥️ Current session' : '📱 Other device'}
						</div>
						<div class="text-gray-500 mt-0.5">
							Created {timeAgo(session.created_at)} &middot; Expires {formatDate(session.expires_at)}
						</div>
					</div>
					{#if !session.current}
						<button
							onclick={() => revokeSession(session.id)}
							class="ml-2 px-2 py-1 text-red-400 hover:bg-red-900/30 rounded text-xs flex-shrink-0"
						>
							Log out
						</button>
					{/if}
				</div>
			{/each}
		</div>

		{#if sessions.length > 1}
			<button
				onclick={revokeAllOthers}
				class="mt-3 text-xs text-red-400 hover:text-red-300 hover:underline"
			>
				Log out all other sessions
			</button>
		{/if}
	{/if}
</div>
