<script lang="ts">
	/**
	 * @file Federation trust management panel
	 * @purpose Admin UI for mesh peers, trust levels, DM policy, and moderation boundaries
	 */
	import { api } from '$lib/api/client';
	import type { MeshPeer, FederationSettings, RemoteUserLookupResponse } from '$lib/api/types';

	let peers = $state<MeshPeer[]>([]);
	let settings = $state<FederationSettings | null>(null);
	let loading = $state(true);
	let saving = $state(false);
	let blocklistText = $state('');
	let toast = $state<{ message: string; type: 'success' | 'error' } | null>(null);

	// Remote user lookup state
	let lookupHostname = $state('');
	let lookupUsername = $state('');
	let lookupLoading = $state(false);
	let lookupResult = $state<RemoteUserLookupResponse | null>(null);

	async function load() {
		loading = true;
		try {
			const [peersRes, settingsRes] = await Promise.all([
				api.get<MeshPeer[]>('/mesh/peers'),
				api.get<FederationSettings>('/admin/federation-settings').catch(() => null),
			]);
			peers = peersRes;
			if (settingsRes) {
				settings = settingsRes;
				blocklistText = settingsRes.instance_blocklist.join('\n');
			}
		} catch (e) {
			showToast('Failed to load federation data', 'error');
		} finally {
			loading = false;
		}
	}

	async function updatePeer(id: string, status?: number, trust_level?: number) {
		try {
			await api.put(`/mesh/peers/${id}`, { status, trust_level });
			await load();
			showToast('Peer updated', 'success');
		} catch (e) {
			showToast('Failed to update peer', 'error');
		}
	}

	async function removePeer(id: string) {
		if (!confirm('Remove this peer from the mesh?')) return;
		try {
			await api.delete(`/mesh/peers/${id}`);
			await load();
			showToast('Peer removed', 'success');
		} catch (e) {
			showToast('Failed to remove peer', 'error');
		}
	}

	async function saveSettings() {
		if (!settings) return;
		saving = true;
		try {
			const payload = {
				allow_remote_dms: settings.allow_remote_dms,
				dm_policy: settings.dm_policy,
				forward_reports: settings.forward_reports,
				instance_blocklist: blocklistText,
			};
			await api.put('/admin/federation-settings', payload);
			showToast('Settings saved', 'success');
		} catch (e) {
			showToast('Failed to save settings', 'error');
		} finally {
			saving = false;
		}
	}

	function showToast(message: string, type: 'success' | 'error') {
		toast = { message, type };
		setTimeout(() => (toast = null), 3000);
	}

	async function lookupRemoteUser() {
		if (!lookupHostname || !lookupUsername) return;
		lookupLoading = true;
		lookupResult = null;
		try {
			const result = await api.get<RemoteUserLookupResponse>(
				`/admin/federation/lookup-user?hostname=${encodeURIComponent(lookupHostname)}&username=${encodeURIComponent(lookupUsername)}`
			);
			lookupResult = result;
		} catch (e: any) {
			lookupResult = {
				hostname: lookupHostname,
				server_verified: false,
				server_public_key: null,
				server_version: null,
				user: null,
				error: e.message ?? 'Lookup failed'
			};
		} finally {
			lookupLoading = false;
		}
	}

	function fmtKey(key: string): string {
		if (!key || key.length < 16) return key;
		return key.slice(0, 8) + '...' + key.slice(-8);
	}

	function fmtDate(d: string | null): string {
		if (!d) return 'Never';
		return new Date(d).toLocaleString();
	}

	$effect(() => {
		load();
	});
</script>

<div class="space-y-6">
	<!-- Toast -->
	{#if toast}
		<div
			class="fixed top-4 right-4 px-4 py-2 rounded text-sm font-medium z-50 transition-opacity"
			class:bg-green-600={toast.type === 'success'}
			class:bg-red-600={toast.type === 'error'}
			class:text-white={true}
		>
			{toast.message}
		</div>
	{/if}

	<!-- Known Peers -->
	<section class="bg-gray-800 rounded-lg overflow-hidden">
		<div class="px-4 py-3 bg-gray-900 border-b border-gray-700 flex items-center justify-between">
			<h3 class="text-sm font-semibold text-white">Known Peers</h3>
			<span class="text-xs text-gray-400">{peers.length} peer{peers.length === 1 ? '' : 's'}</span>
		</div>

		{#if loading}
			<div class="p-8 text-center text-gray-400">Loading peers...</div>
		{:else if peers.length === 0}
			<div class="p-8 text-center text-gray-400">No known peers</div>
		{:else}
			<div class="overflow-x-auto">
				<table class="w-full text-sm">
					<thead class="bg-gray-900">
						<tr class="border-b border-gray-700">
							<th class="px-4 py-2 text-left text-gray-400 font-semibold">Domain</th>
							<th class="px-4 py-2 text-left text-gray-400 font-semibold">Identity</th>
							<th class="px-4 py-2 text-center text-gray-400 font-semibold">Status</th>
							<th class="px-4 py-2 text-center text-gray-400 font-semibold">Trust</th>
							<th class="px-4 py-2 text-left text-gray-400 font-semibold">Last Seen</th>
							<th class="px-4 py-2 text-center text-gray-400 font-semibold">Actions</th>
						</tr>
					</thead>
					<tbody>
						{#each peers as peer (peer.id)}
							<tr class="border-b border-gray-700 hover:bg-gray-700/50 transition">
								<td class="px-4 py-2 text-white font-medium">{peer.hostname}</td>
								<td class="px-4 py-2 text-gray-400 text-xs font-mono" title={peer.public_key}>
									{fmtKey(peer.public_key)}
								</td>
								<td class="px-4 py-2 text-center">
									<span
										class="inline-block px-2 py-0.5 rounded text-xs font-medium"
										class:bg-yellow-900={peer.status === 'pending'}
										class:text-yellow-300={peer.status === 'pending'}
										class:bg-green-900={peer.status === 'active'}
										class:text-green-300={peer.status === 'active'}
										class:bg-red-900={peer.status === 'suspended'}
										class:text-red-300={peer.status === 'suspended'}
									>
										{peer.status}
									</span>
								</td>
								<td class="px-4 py-2 text-center">
									<select
										value={peer.trust_level}
										onchange={(e) => updatePeer(peer.id, undefined, trustNum(e.currentTarget.value))}
										class="bg-gray-700 text-white text-xs rounded px-2 py-1 border border-gray-600 focus:border-gray-500 focus:outline-none"
									>
										<option value="untrusted">Untrusted</option>
										<option value="low">Low</option>
										<option value="medium">Medium</option>
										<option value="high">High</option>
									</select>
								</td>
								<td class="px-4 py-2 text-gray-400 text-xs">{fmtDate(peer.last_seen_at)}</td>
								<td class="px-4 py-2 text-center">
									<div class="flex gap-1 justify-center">
										{#if peer.status !== 'active'}
											<button
												onclick={() => updatePeer(peer.id, 1)}
												class="px-2 py-1 bg-green-700 hover:bg-green-600 text-white text-xs rounded transition-colors"
												title="Allow"
											>
												Allow
											</button>
										{/if}
										{#if peer.status !== 'suspended'}
											<button
												onclick={() => updatePeer(peer.id, 2)}
												class="px-2 py-1 bg-yellow-700 hover:bg-yellow-600 text-white text-xs rounded transition-colors"
												title="Suspend"
											>
												Block
											</button>
										{/if}
										<button
											onclick={() => removePeer(peer.id)}
											class="px-2 py-1 bg-red-700 hover:bg-red-600 text-white text-xs rounded transition-colors"
											title="Remove"
										>
											Remove
										</button>
									</div>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</section>

	<!-- DM Policy -->
	<section class="bg-gray-800 rounded-lg overflow-hidden">
		<div class="px-4 py-3 bg-gray-900 border-b border-gray-700">
			<h3 class="text-sm font-semibold text-white">Remote DM Policy</h3>
		</div>
		<div class="p-4 space-y-4">
			{#if settings}
				<label class="flex items-center gap-3 cursor-pointer">
					<input
						type="checkbox"
						checked={settings.allow_remote_dms}
						onchange={(e) => (settings = { ...settings!, allow_remote_dms: e.currentTarget.checked })}
						class="w-4 h-4 accent-gray-500"
					/>
					<span class="text-sm text-gray-300">Allow remote users to send direct messages</span>
				</label>

				<div>
					<label for="dm-policy" class="block text-xs text-gray-400 mb-1">Default policy for incoming DMs</label>
					<select
						id="dm-policy"
						value={settings.dm_policy}
						onchange={(e) => (settings = { ...settings!, dm_policy: Number(e.currentTarget.value) })}
						class="bg-gray-700 text-white text-sm rounded px-3 py-2 border border-gray-600 focus:border-gray-500 focus:outline-none w-full max-w-xs"
					>
						<option value={0}>Accept all</option>
						<option value={1}>Accept from trusted peers only</option>
						<option value={2}>Accept from mutual spaces only</option>
						<option value={3}>Reject all</option>
					</select>
				</div>
			{:else}
				<div class="text-sm text-gray-400">Loading settings...</div>
			{/if}
		</div>
	</section>

	<!-- Moderation Boundaries -->
	<section class="bg-gray-800 rounded-lg overflow-hidden">
		<div class="px-4 py-3 bg-gray-900 border-b border-gray-700">
			<h3 class="text-sm font-semibold text-white">Moderation Boundaries</h3>
		</div>
		<div class="p-4 space-y-4">
			{#if settings}
				<label class="flex items-center gap-3 cursor-pointer">
					<input
						type="checkbox"
						checked={settings.forward_reports}
						onchange={(e) => (settings = { ...settings!, forward_reports: e.currentTarget.checked })}
						class="w-4 h-4 accent-gray-500"
					/>
					<span class="text-sm text-gray-300">Forward moderation reports to remote instances</span>
				</label>

				<div>
					<label for="blocklist" class="block text-xs text-gray-400 mb-1">Instance blocklist (one domain per line)</label>
					<textarea
						id="blocklist"
						value={blocklistText}
						oninput={(e) => (blocklistText = e.currentTarget.value)}
						rows={5}
						placeholder="bad-actor.example.com&#10;spam.example.com"
						class="bg-gray-700 text-white text-sm rounded px-3 py-2 border border-gray-600 focus:border-gray-500 focus:outline-none w-full font-mono"
					></textarea>
				</div>
			{:else}
				<div class="text-sm text-gray-400">Loading settings...</div>
			{/if}
		</div>
	</section>

	<!-- Remote User Lookup -->
	<section class="bg-gray-800 rounded-lg overflow-hidden">
		<div class="px-4 py-3 bg-gray-900 border-b border-gray-700">
			<h3 class="text-sm font-semibold text-white">Remote User Lookup</h3>
		</div>
		<div class="p-4 space-y-4">
			<p class="text-xs text-gray-400">Look up a user on a remote federated instance and verify their identity.</p>
			<div class="flex flex-wrap gap-2">
				<input
					type="text"
					bind:value={lookupHostname}
					placeholder="Peer hostname (e.g. chat.example.com)"
					class="bg-gray-700 text-white text-sm rounded px-3 py-2 border border-gray-600 focus:border-gray-500 focus:outline-none flex-1 min-w-[200px]"
				/>
				<input
					type="text"
					bind:value={lookupUsername}
					placeholder="Username"
					class="bg-gray-700 text-white text-sm rounded px-3 py-2 border border-gray-600 focus:border-gray-500 focus:outline-none flex-1 min-w-[140px]"
				/>
				<button
					onclick={lookupRemoteUser}
					disabled={lookupLoading || !lookupHostname || !lookupUsername}
					class="px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white text-sm font-medium rounded transition-colors"
				>
					{lookupLoading ? 'Looking up...' : 'Look Up'}
				</button>
			</div>

			{#if lookupResult}
				<div class="border border-gray-700 rounded p-4 space-y-3">
					<!-- Server verification -->
					<div class="flex items-center gap-2">
						<span class="text-xs text-gray-400">Server identity:</span>
						{#if lookupResult.server_verified}
							<span class="inline-block px-2 py-0.5 rounded text-xs font-medium bg-green-900 text-green-300">Verified</span>
						{:else}
							<span class="inline-block px-2 py-0.5 rounded text-xs font-medium bg-red-900 text-red-300">Unverified</span>
						{/if}
						{#if lookupResult.server_public_key}
							<span class="text-xs text-gray-500 font-mono" title={lookupResult.server_public_key}>
								Key: {lookupResult.server_public_key.slice(0, 12)}...{lookupResult.server_public_key.slice(-8)}
							</span>
						{/if}
						{#if lookupResult.server_version}
							<span class="text-xs text-gray-500">v{lookupResult.server_version}</span>
						{/if}
					</div>

					<!-- User info -->
					{#if lookupResult.user}
						<div class="bg-gray-700/50 rounded p-3">
							<div class="text-sm text-white font-medium">{lookupResult.user.username}@{lookupResult.user.server}</div>
							{#if lookupResult.user.display_name}
								<div class="text-xs text-gray-400 mt-1">Display name: {lookupResult.user.display_name}</div>
							{/if}
						</div>
					{/if}

					<!-- Error -->
					{#if lookupResult.error}
						<div class="text-xs text-red-400">{lookupResult.error}</div>
					{/if}
				</div>
			{/if}
		</div>
	</section>

	<!-- Save Bar -->
	<div class="flex justify-end">
		<button
			onclick={saveSettings}
			disabled={saving || !settings}
			class="px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white text-sm font-medium rounded transition-colors"
		>
			{saving ? 'Saving...' : 'Save Settings'}
		</button>
	</div>
</div>

<script lang="ts" module>
	function trustNum(level: string): number {
		const map: Record<string, number> = { untrusted: 0, low: 1, medium: 2, high: 3 };
		return map[level] ?? 2;
	}
</script>
