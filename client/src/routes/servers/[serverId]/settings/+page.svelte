<script lang="ts">
	/**
	 * @file Server settings page
	 * @purpose Rename server, update description, delete server, manage invites, manage roles
	 */
	import { browser } from '$app/environment';
	import { currentServer, fetchServers } from '$lib/stores/servers';
	import api from '$lib/api/client';
	import RoleManager from '$lib/components/modals/RoleManager.svelte';
	import AutomodManager from '$lib/components/modals/AutomodManager.svelte';
	import SlashCommandManager from '$lib/components/modals/SlashCommandManager.svelte';
	import EmojiManager from '$lib/components/modals/EmojiManager.svelte';

	interface Invite { code: string; uses: number; max_uses: number | null; expires_at: string | null; created_at: string; }
	let serverId = $state('');
	let name = $state('');
	let description = $state('');
	let saving = $state(false);
	let deleting = $state(false);
	let error = $state('');
	let success = $state('');
	let invites = $state<Invite[]>([]);
	let invitesLoading = $state(false);
	let showRoleManager = $state(false);
	let showAutomod = $state(false);
	let showSlashCommands = $state(false);
	let showEmojiManager = $state(false);

	if (browser) {
		const match = window.location.pathname.match(/\/servers\/([^/]+)/);
		serverId = match?.[1] ?? '';
	}

	// Sync form with store when server loads
	$effect(() => {
		if ($currentServer) {
			name = $currentServer.name;
			description = $currentServer.description ?? '';
		}
	});

	async function handleSave() {
		if (!name.trim() || !serverId) return;
		saving = true;
		error = '';
		success = '';
		try {
			await api.patch(`/servers/${serverId}`, {
				name: name.trim(),
				description: description.trim() || null
			});
			await fetchServers();
			success = 'Settings saved.';
		} catch (e: any) {
			error = e.message ?? 'Failed to save settings';
		} finally {
			saving = false;
		}
	}

	async function handleDelete() {
		if (!serverId) return;
		if (!confirm(`Delete server "${$currentServer?.name}"? This cannot be undone.`)) return;
		deleting = true;
		error = '';
		try {
			await api.delete(`/servers/${serverId}`);
			window.location.href = '/servers';
		} catch (e: any) {
			error = e.message ?? 'Failed to delete server';
			deleting = false;
		}
	}

	async function loadInvites() {
		if (!serverId) return;
		invitesLoading = true;
		try {
			invites = await api.get<Invite[]>(`/servers/${serverId}/invites`);
		} catch { invites = []; }
		invitesLoading = false;
	}

	async function revokeInvite(code: string) {
		try {
			await api.delete(`/servers/${serverId}/invites/${code}`);
			invites = invites.filter(i => i.code !== code);
		} catch (e: any) {
			error = e.message ?? 'Failed to revoke invite';
		}
	}

	// Load invites when server is known
	$effect(() => { if (serverId) loadInvites(); });
</script>

<div class="flex-1 overflow-y-auto bg-gray-750 p-8">
	<div class="max-w-lg">
		<h1 class="text-xl font-semibold text-white mb-6">Server Settings</h1>

		{#if error}
			<div class="mb-4 px-3 py-2 bg-red-900/40 border border-red-700/50 rounded text-red-300 text-sm">{error}</div>
		{/if}
		{#if success}
			<div class="mb-4 px-3 py-2 bg-green-900/40 border border-green-700/50 rounded text-green-300 text-sm">{success}</div>
		{/if}

		<div class="space-y-4 mb-8">
			<div>
				<label class="block text-xs font-semibold text-gray-400 uppercase mb-1" for="server-name">Server Name</label>
				<input
					id="server-name"
					type="text"
					bind:value={name}
					maxlength="100"
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm placeholder-gray-500 focus:outline-none focus:border-indigo-500"
				/>
			</div>
			<div>
				<label class="block text-xs font-semibold text-gray-400 uppercase mb-1" for="server-description">Description</label>
				<textarea
					id="server-description"
					bind:value={description}
					rows="3"
					maxlength="500"
					placeholder="What's this server about?"
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm placeholder-gray-500 focus:outline-none focus:border-indigo-500 resize-none"
				></textarea>
			</div>
			<button
				onclick={handleSave}
				disabled={saving || !name.trim()}
				class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white text-sm font-medium rounded transition-colors"
			>
				{saving ? 'Saving…' : 'Save Changes'}
			</button>
		</div>

		<div class="border-t border-gray-700 pt-6 mb-6">
		<h2 class="text-sm font-semibold text-gray-400 uppercase mb-3">Invite Links</h2>
		{#if invitesLoading}
			<p class="text-gray-500 text-sm">Loading…</p>
		{:else if invites.length === 0}
			<p class="text-gray-500 text-sm">No active invites.</p>
		{:else}
			<div class="space-y-2">
				{#each invites as inv (inv.code)}
					<div class="flex items-center gap-2 px-3 py-2 bg-gray-900 rounded border border-gray-700">
						<code class="text-indigo-400 text-sm flex-1">{inv.code}</code>
						<span class="text-gray-500 text-xs">{inv.uses} uses</span>
						<button onclick={() => revokeInvite(inv.code)}
							class="px-2 py-1 text-xs text-red-400 hover:bg-gray-800 rounded transition-colors">Revoke</button>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	<div class="border-t border-gray-700 pt-6 mb-6">
		<h2 class="text-sm font-semibold text-gray-400 uppercase mb-2">Roles</h2>
		<button
			onclick={() => (showRoleManager = true)}
			class="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white text-sm rounded transition-colors"
		>
			Manage Roles
		</button>
	</div>

	<div class="border-t border-gray-700 pt-6 mb-6">
		<h2 class="text-sm font-semibold text-gray-400 uppercase mb-2">Moderation</h2>
		<div class="space-y-2">
			<button
				onclick={() => (showAutomod = true)}
				class="w-full px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white text-sm rounded transition-colors text-left"
			>
				AutoMod Settings
			</button>
			<a
				href="/servers/{serverId}/audit-log"
				class="block px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white text-sm rounded transition-colors text-center"
			>
				View Audit Log
			</a>
		</div>
	</div>

	<div class="border-t border-gray-700 pt-6 mb-6">
		<h2 class="text-sm font-semibold text-gray-400 uppercase mb-2">Extensions</h2>
		<div class="space-y-2">
			<button
				onclick={() => (showEmojiManager = true)}
				class="w-full px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white text-sm rounded transition-colors text-left"
			>
				Custom Emojis
			</button>
			<button
				onclick={() => (showSlashCommands = true)}
				class="w-full px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white text-sm rounded transition-colors text-left"
			>
				Slash Commands
			</button>
		</div>
	</div>

	<div class="border-t border-gray-700 pt-6">
		<h2 class="text-sm font-semibold text-red-400 uppercase mb-2">Danger Zone</h2>
		<p class="text-gray-400 text-sm mb-3">Deleting the server is permanent and cannot be undone.</p>
		<button
			onclick={handleDelete}
			disabled={deleting}
			class="px-4 py-2 bg-red-700 hover:bg-red-600 disabled:opacity-50 text-white text-sm font-medium rounded transition-colors"
		>
			{deleting ? 'Deleting…' : 'Delete Server'}
		</button>
	</div>
	</div>
</div>

{#if showRoleManager}
	<RoleManager {serverId} onClose={() => (showRoleManager = false)} />
{/if}

{#if showAutomod}
	<AutomodManager {serverId} onClose={() => (showAutomod = false)} />
{/if}

{#if showEmojiManager}
	<EmojiManager {serverId} onClose={() => (showEmojiManager = false)} />
{/if}

{#if showSlashCommands}
	<SlashCommandManager {serverId} onClose={() => (showSlashCommands = false)} />
{/if}
