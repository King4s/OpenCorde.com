<script lang="ts">
	/**
	 * @file AuthorizedAppsPanel — third-party app authorization management
	 * @purpose List, view permissions, and deauthorize connected apps/bots
	 * @depends api/client, api/types
	 */
	import api from '$lib/api/client';
	import { toastError, toastSuccess } from '$lib/stores/toasts.svelte';

	interface AuthorizedApp {
		id: string;
		application_id: string;
		name: string;
		description: string | null;
		icon_url: string | null;
		scope: string;
		authorized_at: string;
	}

	let apps: AuthorizedApp[] = $state([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	async function loadApps() {
		loading = true;
		error = null;
		try {
			apps = await api.get<AuthorizedApp[]>('/users/@me/authorized-apps');
		} catch (e) {
			error = 'Failed to load authorized apps';
			console.error('[authorized-apps]', e);
		} finally {
			loading = false;
		}
	}

	async function revokeApp(app: AuthorizedApp) {
		if (!confirm(`Revoke access for ${app.name}?`)) return;
		try {
			await api.delete(`/users/@me/authorized-apps/${app.application_id}`);
			apps = apps.filter((a) => a.application_id !== app.application_id);
			toastSuccess(`Revoked access for ${app.name}`);
		} catch (e) {
			toastError('Failed to revoke access');
			console.error('[authorized-apps] revoke:', e);
		}
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleDateString(undefined, {
			year: 'numeric',
			month: 'short',
			day: 'numeric'
		});
	}

	function scopeLabels(scopes: string): string {
		return scopes
			.split(' ')
			.map((s) => {
				switch (s) {
					case 'identify': return 'Profile info';
					case 'guilds': return 'Server list';
					case 'bot': return 'Bot access';
					case 'messages.read': return 'Read messages';
					case 'applications.commands': return 'Slash commands';
					case 'webhook.incoming': return 'Webhooks';
					default: return s;
				}
			})
			.join(', ');
	}

	$effect(() => {
		loadApps();
	});
</script>

<div class="bg-gray-800 rounded-lg p-3 sm:p-4">
	<h2 class="text-sm font-semibold text-gray-400 uppercase mb-3">Authorized Apps</h2>

	{#if loading}
		<p class="text-xs text-gray-500">Loading authorized apps...</p>
	{:else if error}
		<p class="text-xs text-red-400">{error}</p>
		<button onclick={loadApps} class="text-xs text-blue-400 hover:underline mt-1">Retry</button>
	{:else if apps.length === 0}
		<p class="text-xs text-gray-500">No authorized applications.</p>
	{/if}

	{#if apps.length > 0}
		<div class="space-y-2 mt-2">
			{#each apps as app (app.id)}
				<div class="flex items-center justify-between bg-gray-750 rounded p-2 text-xs">
					<div class="flex-1 min-w-0">
						<div class="text-gray-300 truncate font-medium">
							{app.name}
						</div>
						<div class="text-gray-500 mt-0.5">
							{scopeLabels(app.scope)} &middot; Authorized {formatDate(app.authorized_at)}
						</div>
					</div>
					<button
						onclick={() => revokeApp(app)}
						class="ml-2 px-2 py-1 text-red-400 hover:bg-red-900/30 rounded text-xs flex-shrink-0"
					>
						Revoke
					</button>
				</div>
			{/each}
		</div>
	{/if}
</div>
