<script lang="ts">
	/**
	 * @file Server Discovery page
	 * @purpose Browse and join public servers
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

	let servers = $state<DiscoveryServer[]>([]);
	let query = $state('');
	let loading = $state(true);

	async function fetchServers() {
		loading = true;
		try {
			const q = query.trim() ? `&q=${encodeURIComponent(query)}` : '';
			servers = await api.get<DiscoveryServer[]>(`/discover?limit=50${q}`);
		} finally {
			loading = false;
		}
	}

	onMount(fetchServers);

	async function joinServer(serverId: string) {
		goto(`/servers/${serverId}/channels`);
	}

	function getInitials(name: string) {
		return name.slice(0, 2).toUpperCase();
	}
</script>

<div class="discover-page">
	<div class="discover-header">
		<h1>Discover Servers</h1>
		<p class="subtitle">Find communities to join</p>
		<div class="search-bar">
			<input
				bind:value={query}
				placeholder="Search servers..."
				oninput={fetchServers}
			/>
		</div>
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
</div>

<style>
	.discover-page {
		padding: 32px;
		max-width: 900px;
		margin: 0 auto;
	}
	.discover-header {
		text-align: center;
		margin-bottom: 32px;
	}
	.discover-header h1 {
		font-size: 28px;
		font-weight: 800;
		color: #f2f3f5;
		margin: 0 0 8px 0;
	}
	.subtitle {
		color: #b5bac1;
		font-size: 15px;
		margin: 0 0 20px 0;
	}
	.search-bar input {
		width: 100%;
		max-width: 480px;
		background: #1e1f22;
		border: 1px solid #35373c;
		border-radius: 24px;
		color: #dbdee1;
		padding: 12px 20px;
		font-size: 15px;
		outline: none;
		box-sizing: border-box;
	}
	.search-bar input:focus {
		border-color: #5865f2;
	}
	.loading,
	.empty {
		text-align: center;
		color: #b5bac1;
		padding: 40px;
	}
	.server-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: 16px;
	}
	.server-card {
		background: #2b2d31;
		border-radius: 10px;
		padding: 16px;
		cursor: pointer;
		border: 1px solid transparent;
		transition: border-color 0.1s, background 0.1s;
	}
	.server-card:hover {
		border-color: #5865f2;
		background: #313338;
	}
	.server-icon {
		width: 56px;
		height: 56px;
		border-radius: 16px;
		background: #5865f2;
		display: flex;
		align-items: center;
		justify-content: center;
		margin-bottom: 12px;
		overflow: hidden;
		flex-shrink: 0;
	}
	.server-icon img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.initials {
		font-size: 18px;
		font-weight: 800;
		color: white;
	}
	.server-name {
		margin: 0 0 6px 0;
		font-size: 15px;
		font-weight: 700;
		color: #f2f3f5;
	}
	.server-desc {
		margin: 0 0 10px 0;
		font-size: 13px;
		color: #b5bac1;
		line-height: 1.4;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.server-meta {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.member-count {
		font-size: 12px;
		color: #b5bac1;
	}
	.tags {
		display: flex;
		gap: 4px;
		flex-wrap: wrap;
	}
	.tag {
		background: #35373c;
		color: #b5bac1;
		font-size: 11px;
		padding: 2px 8px;
		border-radius: 10px;
	}
</style>
