<script lang="ts">
	/**
	 * @file Server Discovery page
	 * @purpose Browse and join public servers + discover federated mesh network peers
	 */
	import api from '$lib/api/client';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';

	interface DiscoveryServer {
		id: string;
		name: string;
		description: string | null;
		icon_url: string | null;
		member_count: number;
		tags: string | null;
	}

	interface MeshPeer {
		id: string;
		hostname: string;
		status: string;
		last_seen_at: string | null;
	}

	let servers = $state<DiscoveryServer[]>([]);
	let peers = $state<MeshPeer[]>([]);
	let query = $state('');
	let loading = $state(true);
	let tab = $state<'local' | 'network'>('local');

	async function fetchServers() {
		loading = true;
		try {
			const q = query.trim() ? `&q=${encodeURIComponent(query)}` : '';
			servers = await api.get<DiscoveryServer[]>(`/discover?limit=50${q}`);
		} finally {
			loading = false;
		}
	}

	async function fetchPeers() {
		try {
			const allPeers = await api.get<MeshPeer[]>('/mesh/peers');
			peers = allPeers.filter(p => p.status === 'active');
		} catch {
			peers = [];
		}
	}

	onMount(() => {
		fetchServers();
		fetchPeers();
	});

	async function joinServer(serverId: string) {
		goto(`/servers/${serverId}/channels`);
	}

	function visitPeer(hostname: string) {
		window.open(`https://${hostname}`, '_blank', 'noopener,noreferrer');
	}

	function getInitials(name: string) {
		return name.slice(0, 2).toUpperCase();
	}

	function timeSince(isoDate: string | null): string {
		if (!isoDate) return 'never';
		const ms = Date.now() - new Date(isoDate).getTime();
		const h = Math.floor(ms / 3600000);
		if (h < 1) return 'just now';
		if (h < 24) return `${h}h ago`;
		return `${Math.floor(h / 24)}d ago`;
	}
</script>

<div class="discover-page">
	<div class="discover-header">
		<h1>Discover</h1>
		<p class="subtitle">Find communities on this server and across the network</p>

		<!-- Tab switcher -->
		<div class="tabs">
			<button
				class="tab-btn"
				class:active={tab === 'local'}
				onclick={() => { tab = 'local'; }}
			>
				This Server
			</button>
			<button
				class="tab-btn"
				class:active={tab === 'network'}
				onclick={() => { tab = 'network'; fetchPeers(); }}
			>
				Federation Network
				{#if peers.length > 0}
					<span class="badge">{peers.length}</span>
				{/if}
			</button>
		</div>
	</div>

	{#if tab === 'local'}
		<!-- Local server discovery -->
		<div class="search-bar">
			<input
				bind:value={query}
				placeholder="Search servers..."
				oninput={fetchServers}
			/>
		</div>

		{#if loading}
			<p class="loading">Loading...</p>
		{:else if servers.length === 0}
			<p class="empty">No public servers found.</p>
		{:else}
			<div class="server-grid">
				{#each servers as server (server.id)}
					<div
						class="server-card"
						role="button"
						tabindex="0"
						onclick={() => joinServer(server.id)}
						onkeydown={(e) => e.key === 'Enter' && joinServer(server.id)}
					>
						<div class="server-icon">
							{#if server.icon_url}
								<img src={server.icon_url} alt={server.name} />
							{:else}
								<span class="initials">{getInitials(server.name)}</span>
							{/if}
						</div>
						<div class="server-info">
							<h3 class="server-name">{server.name}</h3>
							{#if server.description}
								<p class="server-desc">{server.description}</p>
							{/if}
							<div class="server-meta">
								<span class="member-count">👥 {server.member_count.toLocaleString()} members</span>
								{#if server.tags}
									<div class="tags">
										{#each server.tags.split(',').slice(0, 3) as tag}
											<span class="tag">{tag.trim()}</span>
										{/each}
									</div>
								{/if}
							</div>
						</div>
					</div>
				{/each}
			</div>
		{/if}

	{:else}
		<!-- Federation network tab -->
		<div class="network-header">
			<p class="network-desc">
				Other OpenCorde servers in the federation network. You can message users on these servers
				using <strong>username@hostname</strong> in the DM search.
			</p>
		</div>

		{#if peers.length === 0}
			<div class="empty-network">
				<p class="empty">No federated servers connected yet.</p>
				<p class="empty-hint">Servers that introduce themselves via the federation protocol will appear here.</p>
			</div>
		{:else}
			<div class="peer-list">
				{#each peers as peer (peer.id)}
					<div class="peer-card">
						<div class="peer-icon">
							<span class="peer-initial">{peer.hostname[0].toUpperCase()}</span>
						</div>
						<div class="peer-info">
							<h3 class="peer-hostname">{peer.hostname}</h3>
							<p class="peer-meta">
								<span class="status-dot active"></span>
								Active · Last seen {timeSince(peer.last_seen_at)}
							</p>
							<p class="peer-hint">DM users: <code>username@{peer.hostname}</code></p>
						</div>
						<div class="peer-actions">
							<button
								onclick={() => visitPeer(peer.hostname)}
								class="visit-btn"
								title="Open in new tab"
							>
								Visit ↗
							</button>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	{/if}
</div>

<style>
	.discover-page { padding: 32px; max-width: 900px; margin: 0 auto; }
	.discover-header { text-align: center; margin-bottom: 24px; }
	.discover-header h1 { font-size: 28px; font-weight: 800; color: #f2f3f5; margin: 0 0 8px 0; }
	.subtitle { color: #b5bac1; font-size: 15px; margin: 0 0 20px 0; }
	.tabs {
		display: inline-flex; gap: 4px; justify-content: center;
		background: #1e1f22; border-radius: 24px; padding: 4px;
	}
	.tab-btn {
		background: none; border: none; color: #b5bac1; font-size: 14px;
		font-weight: 500; padding: 8px 20px; border-radius: 20px; cursor: pointer;
		transition: background 0.15s, color 0.15s; display: flex; align-items: center; gap: 6px;
	}
	.tab-btn:hover { background: #35373c; color: #dbdee1; }
	.tab-btn.active { background: #5865f2; color: white; }
	.badge {
		background: #ed4245; color: white; font-size: 10px; font-weight: 700;
		padding: 1px 6px; border-radius: 10px; min-width: 18px; text-align: center;
	}
	.search-bar { text-align: center; margin-bottom: 24px; }
	.search-bar input {
		width: 100%; max-width: 480px; background: #1e1f22; border: 1px solid #35373c;
		border-radius: 24px; color: #dbdee1; padding: 12px 20px; font-size: 15px;
		outline: none; box-sizing: border-box;
	}
	.search-bar input:focus { border-color: #5865f2; }
	.loading, .empty { text-align: center; color: #b5bac1; padding: 40px; }
	.empty-hint { text-align: center; color: #6b6d73; font-size: 13px; margin-top: -24px; }
	.server-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 16px; }
	.server-card {
		background: #2b2d31; border-radius: 10px; padding: 16px; cursor: pointer;
		border: 1px solid transparent; transition: border-color 0.1s, background 0.1s;
	}
	.server-card:hover { border-color: #5865f2; background: #313338; }
	.server-icon {
		width: 56px; height: 56px; border-radius: 16px; background: #5865f2;
		display: flex; align-items: center; justify-content: center;
		margin-bottom: 12px; overflow: hidden; flex-shrink: 0;
	}
	.server-icon img { width: 100%; height: 100%; object-fit: cover; }
	.initials { font-size: 18px; font-weight: 800; color: white; }
	.server-name { margin: 0 0 6px 0; font-size: 15px; font-weight: 700; color: #f2f3f5; }
	.server-desc {
		margin: 0 0 10px 0; font-size: 13px; color: #b5bac1; line-height: 1.4;
		display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2;
		-webkit-box-orient: vertical; overflow: hidden;
	}
	.server-meta { display: flex; flex-direction: column; gap: 6px; }
	.member-count { font-size: 12px; color: #b5bac1; }
	.tags { display: flex; gap: 4px; flex-wrap: wrap; }
	.tag { background: #35373c; color: #b5bac1; font-size: 11px; padding: 2px 8px; border-radius: 10px; }
	/* Federation network */
	.network-header { margin-bottom: 20px; }
	.network-desc {
		color: #b5bac1; font-size: 14px; line-height: 1.5; background: #2b2d31;
		border: 1px solid #35373c; border-radius: 8px; padding: 12px 16px; text-align: center;
	}
	.peer-list { display: flex; flex-direction: column; gap: 8px; }
	.peer-card {
		background: #2b2d31; border-radius: 10px; padding: 16px;
		border: 1px solid #35373c; display: flex; align-items: center; gap: 14px;
	}
	.peer-icon {
		width: 48px; height: 48px; border-radius: 12px;
		background: linear-gradient(135deg, #5865f2, #57f287);
		display: flex; align-items: center; justify-content: center; flex-shrink: 0;
	}
	.peer-initial { font-size: 20px; font-weight: 800; color: white; }
	.peer-info { flex: 1; min-width: 0; }
	.peer-hostname { margin: 0 0 3px 0; font-size: 15px; font-weight: 700; color: #f2f3f5; }
	.peer-meta { margin: 0 0 4px 0; font-size: 12px; color: #b5bac1; display: flex; align-items: center; gap: 5px; }
	.status-dot { width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0; }
	.status-dot.active { background: #23a559; }
	.peer-hint { margin: 0; font-size: 12px; color: #6b6d73; }
	.peer-hint code { background: #1e1f22; padding: 1px 4px; border-radius: 3px; color: #b5bac1; }
	.peer-actions { flex-shrink: 0; }
	.visit-btn {
		background: #35373c; color: #dbdee1; border: none; border-radius: 6px;
		padding: 6px 14px; font-size: 13px; cursor: pointer; transition: background 0.1s;
	}
	.visit-btn:hover { background: #5865f2; color: white; }
</style>
