<script lang="ts">
	/**
	 * @file Upgrade & Migration Panel
	 * @purpose Admin controls: version check, migration status, upgrade flow
	 */
	import api from '$lib/api/client';
	import type { UpgradeStatus, UpgradeCheckResponse, MigrateResponse, RollbackGuidance } from '$lib/api/types';

	let status = $state<UpgradeStatus | null>(null);
	let rollback = $state<RollbackGuidance | null>(null);
	let loading = $state(false);
	let checkLoading = $state(false);
	let migrateLoading = $state(false);
	let rollbackLoading = $state(false);
	let error = $state('');
	let successMsg = $state('');
	let migrateResult = $state<MigrateResponse | null>(null);
	let showRollback = $state(false);

	$effect(() => {
		loadStatus();
	});

	async function loadStatus() {
		try {
			loading = true;
			status = await api.get<UpgradeStatus>('/admin/upgrade/status');
			error = '';
		} catch (e: any) {
			error = e.message?.includes('403') ? 'Access denied. Admin privileges required.' : (e.message ?? 'Failed to load upgrade status');
			status = null;
		} finally { loading = false; }
	}

	async function checkForUpdates() {
		checkLoading = true;
		error = '';
		successMsg = '';
		try {
			const result = await api.post<UpgradeCheckResponse>('/admin/upgrade/check');
			if (result.upgrade_available) {
				successMsg = `Update available: ${result.latest}`;
				if (result.changelog_url) {
					successMsg += ' — release notes linked below.';
				}
			} else {
				successMsg = 'You are running the latest version.';
			}
			await loadStatus();
		} catch (e: any) {
			error = e.message ?? 'Failed to check for updates';
		} finally { checkLoading = false; }
	}

	async function runMigrations() {
		if (migrateLoading) return;
		const confirmed = confirm(
			'Run pending database migrations?\n\nThis will apply structural changes to the database. A backup is recommended before proceeding.'
		);
		if (!confirmed) return;

		migrateLoading = true;
		error = '';
		successMsg = '';
		migrateResult = null;
		try {
			const result = await api.post<MigrateResponse>('/admin/upgrade/migrate');
			migrateResult = result;
			if (result.success) {
				successMsg = `Successfully applied ${result.applied_count} migration(s).`;
			} else {
				error = result.error ?? 'Migration failed';
			}
			await loadStatus();
		} catch (e: any) {
			error = e.message ?? 'Failed to run migrations';
		} finally { migrateLoading = false; }
	}

	async function loadRollbackGuidance() {
		rollbackLoading = true;
		error = '';
		try {
			rollback = await api.post<RollbackGuidance>('/admin/upgrade/rollback');
			showRollback = true;
		} catch (e: any) {
			error = e.message ?? 'Failed to load rollback guidance';
		} finally { rollbackLoading = false; }
	}

	function statusBadge(available: boolean): string {
		return available ? 'Update Available' : 'Up to Date';
	}

	function statusColor(available: boolean): string {
		return available ? 'text-amber-400' : 'text-green-400';
	}

	function timeAgo(iso: string | null): string {
		if (!iso) return 'Never';
		const then = new Date(iso).getTime();
		const now = Date.now();
		const diff = Math.floor((now - then) / 1000);
		if (diff < 60) return 'Just now';
		if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
		if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
		return `${Math.floor(diff / 86400)}d ago`;
	}
</script>

<div class="bg-gray-800 rounded-lg p-4 sm:p-6 space-y-6">
	<!-- Error / Success messages -->
	{#if error}
		<div class="px-4 py-3 bg-gray-900/40 border border-gray-700/50 rounded text-gray-300 text-sm">{error}</div>
	{/if}
	{#if successMsg}
		<div class="px-4 py-3 bg-green-900/20 border border-green-800/40 rounded text-green-300 text-sm">{successMsg}</div>
	{/if}

	<!-- Version Panel -->
	<div class="border border-gray-700 rounded-lg p-4 sm:p-6">
		<h3 class="text-white font-medium text-lg mb-4">Instance Version</h3>

		{#if loading}
			<div class="text-gray-400 text-sm">Loading status...</div>
		{:else if status}
			<div class="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-4">
				<div class="bg-gray-900/60 rounded p-4">
					<div class="text-gray-400 text-xs uppercase mb-1">Current Version</div>
					<div class="text-xl font-mono text-white">{status.current_version}</div>
				</div>
				<div class="bg-gray-900/60 rounded p-4">
					<div class="text-gray-400 text-xs uppercase mb-1">Latest Available</div>
					<div class="text-xl font-mono {statusColor(status.upgrade_available)}">
						{status.latest_version ?? 'Unknown'}
					</div>
				</div>
			</div>

			<div class="flex flex-wrap items-center gap-3 mb-3">
				<span class="text-sm font-medium {statusColor(status.upgrade_available)}">
					{statusBadge(status.upgrade_available)}
				</span>
				<button
					onclick={checkForUpdates}
					disabled={checkLoading}
					class="px-4 py-2 bg-gray-600 hover:bg-gray-500 disabled:bg-gray-600 text-white rounded text-sm font-medium transition-colors"
				>
					{checkLoading ? 'Checking...' : 'Check for Updates'}
				</button>
			</div>

			<!-- Auto-check & last checked info -->
			<div class="flex flex-wrap items-center gap-4 text-xs text-gray-500 mb-3">
				<span>
					Auto-check: <span class={status.auto_check_enabled ? 'text-green-400' : 'text-amber-400'}>
						{status.auto_check_enabled ? 'Enabled' : 'Disabled'}
					</span>
				</span>
				<span>Last checked: {timeAgo(status.last_checked_at)}</span>
			</div>

			{#if status.upgrade_available}
				<div class="bg-amber-900/20 border border-amber-800/30 rounded p-3 text-amber-300 text-sm mb-3">
					An upgrade is available from {status.current_version} → {status.latest_version}.
					Review the changelog before upgrading. Run database migrations after upgrading the binary.
				</div>
			{/if}

			{#if status.changelog_url}
				<a
					href={status.changelog_url}
					target="_blank"
					rel="noopener noreferrer"
					class="inline-flex items-center gap-1.5 px-3 py-1.5 bg-gray-700 hover:bg-gray-600 text-gray-200 rounded text-sm transition-colors"
				>
					<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
					</svg>
					View Release Notes
				</a>
			{/if}

			{#if status.latest_version && !status.changelog_url && status.upgrade_available}
				<div class="mt-3 text-gray-500 text-xs">
					No changelog URL available. Check the repository for release information.
				</div>
			{/if}
		{:else}
			<div class="text-gray-500 text-sm">Unable to load version information.</div>
		{/if}
	</div>

	<!-- Migration Status Panel -->
	<div class="border border-gray-700 rounded-lg p-4 sm:p-6">
		<div class="flex flex-wrap items-center justify-between gap-3 mb-4">
			<h3 class="text-white font-medium text-lg">Database Migrations</h3>
			<button
				onclick={runMigrations}
				disabled={migrateLoading || !status || status.pending_migrations === 0}
				class="px-4 py-2 bg-gray-600 hover:bg-gray-500 disabled:bg-gray-700 disabled:text-gray-500 text-white rounded text-sm font-medium transition-colors"
			>
				{migrateLoading ? 'Running...' : 'Run Migrations'}
			</button>
		</div>

		{#if loading}
			<div class="text-gray-400 text-sm">Loading migration status...</div>
		{:else if status}
			<div class="grid grid-cols-2 sm:grid-cols-3 gap-3 mb-4">
				<div class="bg-gray-900/60 rounded p-3">
					<div class="text-gray-400 text-xs uppercase mb-1">Total</div>
					<div class="text-xl font-mono text-white">{status.total_migrations}</div>
				</div>
				<div class="bg-gray-900/60 rounded p-3">
					<div class="text-gray-400 text-xs uppercase mb-1">Applied</div>
					<div class="text-xl font-mono text-green-400">{status.applied_migrations}</div>
				</div>
				<div class="bg-gray-900/60 rounded p-3">
					<div class="text-gray-400 text-xs uppercase mb-1">Pending</div>
					<div class="text-xl font-mono {status.pending_migrations > 0 ? 'text-amber-400' : 'text-gray-300'}">
						{status.pending_migrations}
					</div>
				</div>
			</div>

			{#if status.last_migration_at}
				<div class="text-gray-500 text-xs mb-3">
					Last migration applied: {new Date(status.last_migration_at).toLocaleString()}
				</div>
			{/if}

			{#if status.pending_migration_names.length > 0}
				<div class="mb-4">
					<div class="text-gray-400 text-xs uppercase mb-2">Pending Migrations</div>
					<div class="space-y-1 max-h-48 overflow-y-auto">
						{#each status.pending_migration_names as name}
							<div class="bg-gray-900/60 rounded px-3 py-1.5 text-sm font-mono text-amber-300">
								{name}
							</div>
						{/each}
					</div>
				</div>
			{:else}
				<div class="text-green-400 text-sm mb-4">All migrations have been applied.</div>
			{/if}

			{#if status.migration_in_progress}
				<div class="bg-blue-900/20 border border-blue-800/30 rounded p-3 text-blue-300 text-sm mb-3">
					A migration is currently in progress. Please wait...
				</div>
			{/if}

			<!-- Migration result feedback -->
			{#if migrateResult}
				<div class="border border-gray-700 rounded p-3 mb-3 {migrateResult.success ? 'border-green-800/40 bg-green-900/10' : 'border-red-800/40 bg-red-900/10'}">
					<div class="text-sm font-medium {migrateResult.success ? 'text-green-400' : 'text-red-400'} mb-1">
						{migrateResult.success ? 'Migration Successful' : 'Migration Failed'}
					</div>
					{#if migrateResult.applied_count > 0}
						<div class="text-xs text-gray-400 mb-1">{migrateResult.applied_count} migration(s) applied</div>
						<div class="space-y-0.5">
							{#each migrateResult.applied_names as name}
								<div class="text-xs font-mono text-green-300">{name}</div>
							{/each}
						</div>
					{/if}
					{#if migrateResult.error}
						<div class="text-xs text-red-300 mt-1 font-mono">{migrateResult.error}</div>
					{/if}
				</div>
			{/if}
		{:else}
			<div class="text-gray-500 text-sm">Unable to load migration information.</div>
		{/if}
	</div>

	<!-- Rollback Guidance Panel -->
	<div class="border border-gray-700 rounded-lg p-4 sm:p-6">
		<div class="flex flex-wrap items-center justify-between gap-3 mb-4">
			<h3 class="text-white font-medium text-lg">Rollback Guidance</h3>
			<button
				onclick={loadRollbackGuidance}
				disabled={rollbackLoading}
				class="px-4 py-2 bg-gray-600 hover:bg-gray-500 disabled:bg-gray-600 text-white rounded text-sm font-medium transition-colors"
			>
				{rollbackLoading ? 'Loading...' : showRollback ? 'Refresh' : 'Show Guidance'}
			</button>
		</div>

		{#if showRollback && rollback}
			<div class="bg-gray-900/60 rounded p-4 mb-3">
				<div class="text-gray-300 text-sm mb-3">{rollback.guidance}</div>

				<div class="space-y-2">
					<div class="text-gray-400 text-xs uppercase mb-1">Recovery Steps</div>
					{#each rollback.steps as step, i}
						<div class="flex items-start gap-2 text-sm">
							<span class="text-gray-500 font-mono flex-shrink-0">{i + 1}.</span>
							<span class="text-gray-300">{step}</span>
						</div>
					{/each}
				</div>

				{#if rollback.backup_available}
					<div class="mt-3 px-3 py-2 bg-green-900/20 border border-green-800/30 rounded text-green-300 text-xs">
						Backups are available — you can restore from the Backups tab if needed.
					</div>
				{:else}
					<div class="mt-3 px-3 py-2 bg-amber-900/20 border border-amber-800/30 rounded text-amber-300 text-xs">
						No backups found. Create a backup before running migrations to enable safe rollback.
					</div>
				{/if}
			</div>
		{:else if !showRollback}
			<div class="text-gray-500 text-sm">Click "Show Guidance" to view rollback instructions and recovery steps.</div>
		{/if}
	</div>
</div>
