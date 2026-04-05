<script lang="ts">
	/**
	 * @file Server Discovery page
	 * @purpose Browse and join public spaces + discover federated mesh network nodes
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

	let spaces = $state<DiscoveryServer[]>([]);
	let peers = $state<MeshPeer[]>([]);
	let query = $state('');
	let loading = $state(true);
	let tab = $state<'local' | 'network'>('local');

	async function fetchSpaces() {
		loading = true;
		try {
			const q = query.trim() ? `&q=${encodeURIComponent(query)}` : '';
			spaces = await api.get<DiscoveryServer[]>(`/discover?limit=50${q}`);
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
		fetchSpaces();
		fetchPeers();
	});

	async function joinServer(spaceId: string) {
		goto(`/servers/${spaceId}/channels`);
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
		<p class="subtitle">Find communities on this instance and across the network</p>

		<!-- Tab switcher -->
		<div class="tabs">
			<button
				class="tab-btn"
				class:active={tab === 'local'}
				onclick={() => { tab = 'local'; }}
			>
				This Instance
			</button>
			<button
				class="tab-btn"
				class:active={tab === 'network'}
				onclick={() => { tab = 'network'; fetchPeers(); }}
			>
				Network
				{#if peers.length > 0}
					<span class="badge">{peers.length}</span>
				{/if}
			</button>
		</div>
	</div>

	{#if tab === 'local'}
		<!-- Local space discovery -->
		<div class="search-bar">
			<input
				bind:value={query}
				placeholder="Search spaces..."
				oninput={fetchSpaces}
			/>
		</div>

		{#if loading}
			<p class="loading">Loading...</p>
		{:else if spaces.length === 0}
			<p class="empty">No public spaces found.</p>
		{:else}
			<div class="space-grid">
				{#each spaces as space (space.id)}
					<div
						class="space-card"
						role="button"
						tabindex="0"
						onclick={() => joinServer(space.id)}
						onkeydown={(e) => e.key === 'Enter' && joinServer(space.id)}
					>
						<div class="space-icon">
							{#if space.icon_url}
								<img src={space.icon_url} alt={space.name} />
							{:else}
								<span class="initials">{getInitials(space.name)}</span>
							{/if}
						</div>
						<div class="space-info">
							<h3 class="space-name">{space.name}</h3>
							{#if space.description}
								<p class="space-desc">{space.description}</p>
							{/if}
							<div class="space-meta">
								<span class="member-count">👥 {space.member_count.toLocaleString()} members</span>
								{#if space.tags}
									<div class="tags">
										{#each space.tags.split(',').slice(0, 3) as tag}
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
				Other OpenCorde instances in the network. You can message users on these spaces
				using <strong>username@hostname</strong> in the DM search.
			</p>
		</div>

		{#if peers.length === 0}
			<div class="empty-network">
				<p class="empty">No nodes connected yet.</p>
				<p class="empty-hint">Instances that introduce themselves via the federation protocol will appear here.</p>
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
	.tab-btn.active { background: #e5e7eb; color: white; }
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
	.search-bar input:focus { border-color: #e5e7eb; }
	.loading, .empty { text-align: center; color: #b5bac1; padding: 40px; }
	.empty-hint { text-align: center; color: #6b6d73; font-size: 13px; margin-top: -24px; }
	.space-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 16px; }
	.space-card {
		background: #2b2d31; border-radius: 10px; padding: 16px; cursor: pointer;
		border: 1px solid transparent; transition: border-color 0.1s, background 0.1s;
	}
	.space-card:hover { border-color: #e5e7eb; background: #313338; }
	.space-icon {
		width: 56px; height: 56px; border-radius: 16px; background: #e5e7eb;
		display: flex; align-items: center; justify-content: center;
		margin-bottom: 12px; overflow: hidden; flex-shrink: 0;
	}
	.space-icon img { width: 100%; height: 100%; object-fit: cover; }
	.initials { font-size: 18px; font-weight: 800; color: white; }
	.space-name { margin: 0 0 6px 0; font-size: 15px; font-weight: 700; color: #f2f3f5; }
	.space-desc {
		margin: 0 0 10px 0; font-size: 13px; color: #b5bac1; line-height: 1.4;
		display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2;
		-webkit-box-orient: vertical; overflow: hidden;
	}
	.space-meta { display: flex; flex-direction: column; gap: 6px; }
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
		background: linear-gradient(135deg, #e5e7eb, #57f287);
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
	.visit-btn:hover { background: #e5e7eb; color: white; }

	@media (max-width: 744px) {
		.discover-page { padding: 20px 16px 24px; }
		.discover-header { text-align: left; margin-bottom: 20px; }
		.discover-header h1 { font-size: 24px; }
		.subtitle { font-size: 14px; margin-bottom: 16px; }
		.tabs {
			display: flex;
			width: 100%;
			gap: 6px;
		}
		.tab-btn {
			flex: 1 1 0;
			justify-content: center;
			padding: 10px 12px;
			font-size: 13px;
		}
		.search-bar { text-align: left; margin-bottom: 18px; }
		.search-bar input { max-width: none; padding: 11px 14px; font-size: 14px; }
		.space-grid { grid-template-columns: 1fr; gap: 12px; }
		.space-card { padding: 14px; }
		.peer-card {
			padding: 14px;
			gap: 12px;
			align-items: flex-start;
			flex-direction: column;
		}
		.peer-actions { width: 100%; }
		.visit-btn { width: 100%; }
	}

	@media (max-width: 430px) {
		.discover-page { padding: 16px 12px 20px; }
		.discover-header h1 { font-size: 22px; }
		.tab-btn { padding: 9px 10px; font-size: 12px; }
		.space-icon { width: 48px; height: 48px; }
		.initials { font-size: 16px; }
	}
</style>
