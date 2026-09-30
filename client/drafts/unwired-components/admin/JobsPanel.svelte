<script lang="ts">
	/**
	 * @file Jobs Panel
	 * @purpose Background job monitoring dashboard — list jobs, retry failures, pause/resume queues
	 * @version 1.0.0
	 */
	import api from '$lib/api/client';
	import type { JobInfo, JobDetail, JobsResponse } from '$lib/api/types';

	let jobs = $state<JobInfo[]>([]);
	let metrics = $state<JobsResponse['metrics'] | null>(null);
	let loading = $state(false);
	let error = $state('');
	let selectedJob = $state<JobDetail | null>(null);
	let selectedLoading = $state(false);
	let actionLoading = $state<string | null>(null);
	let retryAllLoading = $state(false);

	async function loadJobs() {
		loading = true;
		error = '';
		try {
			const resp = await api.get<JobsResponse>('/admin/jobs');
			jobs = resp.jobs;
			metrics = resp.metrics;
		} catch (e: any) {
			error = e.message?.includes('403')
				? 'Access denied. Admin privileges required.'
				: (e.message ?? 'Failed to load jobs');
			jobs = [];
			metrics = null;
		} finally {
			loading = false;
		}
	}

	async function viewJob(jobId: string) {
		selectedLoading = true;
		try {
			selectedJob = await api.get<JobDetail>(`/admin/jobs/${jobId}`);
		} catch (e: any) {
			error = e.message ?? 'Failed to load job details';
		} finally {
			selectedLoading = false;
		}
	}

	async function retryJob(jobId: string) {
		actionLoading = jobId;
		try {
			await api.post(`/admin/jobs/${jobId}/retry`);
			await loadJobs();
			if (selectedJob?.id === jobId) await viewJob(jobId);
		} catch (e: any) {
			error = e.message ?? 'Failed to retry job';
		} finally {
			actionLoading = null;
		}
	}

	async function pauseJob(jobId: string) {
		actionLoading = jobId;
		try {
			await api.post(`/admin/jobs/${jobId}/pause`);
			await loadJobs();
			if (selectedJob?.id === jobId) await viewJob(jobId);
		} catch (e: any) {
			error = e.message ?? 'Failed to pause job';
		} finally {
			actionLoading = null;
		}
	}

	async function resumeJob(jobId: string) {
		actionLoading = jobId;
		try {
			await api.post(`/admin/jobs/${jobId}/resume`);
			await loadJobs();
			if (selectedJob?.id === jobId) await viewJob(jobId);
		} catch (e: any) {
			error = e.message ?? 'Failed to resume job';
		} finally {
			actionLoading = null;
		}
	}

	async function retryAllFailed() {
		retryAllLoading = true;
		try {
			await api.post('/admin/jobs/retry-failed');
			await loadJobs();
		} catch (e: any) {
			error = e.message ?? 'Failed to retry all jobs';
		} finally {
			retryAllLoading = false;
		}
	}

	function statusBadge(status: string): string {
		switch (status) {
			case 'running': return 'bg-blue-500/20 text-blue-400 border-blue-500/30';
			case 'failed': return 'bg-red-500/20 text-red-400 border-red-500/30';
			case 'paused': return 'bg-amber-500/20 text-amber-400 border-amber-500/30';
			case 'completed': return 'bg-green-500/20 text-green-400 border-green-500/30';
			default: return 'bg-gray-600/30 text-gray-400 border-gray-500/30';
		}
	}

	function statusDot(status: string): string {
		switch (status) {
			case 'running': return 'bg-blue-400';
			case 'failed': return 'bg-red-400';
			case 'paused': return 'bg-amber-400';
			case 'completed': return 'bg-green-400';
			default: return 'bg-gray-500';
		}
	}

	function formatDuration(ms: number | null): string {
		if (ms === null || ms === undefined) return '—';
		if (ms < 1000) return `${ms} ms`;
		if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
		return `${(ms / 60000).toFixed(1)}m`;
	}

	function formatTimestamp(ts: string | null): string {
		if (!ts) return '—';
		try {
			return new Date(ts).toLocaleString();
		} catch {
			return ts;
		}
	}

	function formatRate(rate: number): string {
		return `${(rate * 100).toFixed(1)}%`;
	}

	$effect(() => {
		loadJobs();
	});
</script>

<div class="space-y-4">
	<!-- Header -->
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<h2 class="text-white font-medium text-lg">Background Jobs</h2>
			<p class="text-gray-500 text-sm mt-0.5">
				Monitor and manage all background tasks
			</p>
		</div>
		<div class="flex gap-2">
			{#if metrics && metrics.failed > 0}
				<button
					onclick={retryAllFailed}
					disabled={retryAllLoading}
					class="px-3 py-1.5 text-xs rounded bg-amber-700/40 text-amber-300 hover:bg-amber-700/60 disabled:opacity-50 transition-colors"
				>
					{retryAllLoading ? 'Retrying…' : `Retry All Failed (${metrics.failed})`}
				</button>
			{/if}
			<button
				onclick={loadJobs}
				disabled={loading}
				class="px-3 py-1.5 text-xs rounded bg-gray-700 text-gray-200 hover:bg-gray-600 disabled:opacity-50 transition-colors"
			>
				{loading ? 'Loading…' : '↻ Refresh'}
			</button>
		</div>
	</div>

	{#if error}
		<div class="px-4 py-3 bg-gray-900/40 border border-gray-700/50 rounded text-gray-300 text-sm">{error}</div>
	{/if}

	<!-- Metrics cards -->
	{#if metrics}
		<div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3">
			<div class="bg-gray-800 rounded-lg p-3">
				<div class="text-gray-400 text-xs uppercase">Total Jobs</div>
				<div class="text-xl font-bold text-white mt-0.5">{metrics.total_jobs}</div>
			</div>
			<div class="bg-gray-800 rounded-lg p-3">
				<div class="text-gray-400 text-xs uppercase">Running</div>
				<div class="text-xl font-bold text-blue-400 mt-0.5">{metrics.running}</div>
			</div>
			<div class="bg-gray-800 rounded-lg p-3">
				<div class="text-gray-400 text-xs uppercase">Failed</div>
				<div class="text-xl font-bold text-red-400 mt-0.5">{metrics.failed}</div>
			</div>
			<div class="bg-gray-800 rounded-lg p-3">
				<div class="text-gray-400 text-xs uppercase">Paused</div>
				<div class="text-xl font-bold text-amber-400 mt-0.5">{metrics.paused}</div>
			</div>
			<div class="bg-gray-800 rounded-lg p-3">
				<div class="text-gray-400 text-xs uppercase">Runs (1h)</div>
				<div class="text-xl font-bold text-white mt-0.5">{metrics.runs_last_hour}</div>
			</div>
			<div class="bg-gray-800 rounded-lg p-3">
				<div class="text-gray-400 text-xs uppercase">Failure Rate</div>
				<div class="text-xl font-bold text-white mt-0.5">{formatRate(metrics.failure_rate)}</div>
			</div>
		</div>
	{/if}

	<!-- Jobs table / list -->
	{#if loading}
		<div class="py-8 text-center text-gray-500 text-sm">Loading jobs...</div>
	{:else if jobs.length === 0}
		<div class="py-8 text-center text-gray-500 text-sm">No background jobs registered.</div>
	{:else}
		<div class="space-y-2">
			{#each jobs as job}
				<button
					onclick={() => viewJob(job.id)}
					class="w-full text-left bg-gray-800 rounded-lg border border-gray-700 hover:border-gray-600 p-4 transition-colors"
				>
					<div class="flex flex-wrap items-center justify-between gap-2">
						<div class="flex items-center gap-3 min-w-0">
							<span class="w-2 h-2 rounded-full {statusDot(job.status)} shrink-0"></span>
							<div class="min-w-0">
								<div class="text-white text-sm font-medium truncate">{job.name}</div>
								<div class="text-gray-500 text-xs mt-0.5">{job.id} · {job.queue}</div>
							</div>
						</div>
						<div class="flex items-center gap-2 shrink-0">
							<span class="px-2 py-0.5 text-xs rounded border {statusBadge(job.status)}">{job.status}</span>
							{#if job.run_count > 0}
								<span class="text-gray-500 text-xs">{job.run_count} runs</span>
							{/if}
						</div>
					</div>
					<div class="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs text-gray-500">
						{#if job.last_run_at}<span>Last: {formatTimestamp(job.last_run_at)}</span>{/if}
						{#if job.next_run_at}<span>Next: {formatTimestamp(job.next_run_at)}</span>{/if}
						{#if job.avg_duration_ms !== null}<span>Avg: {formatDuration(job.avg_duration_ms)}</span>{/if}
						{#if job.last_error}
							<span class="text-red-400/70 truncate max-w-xs" title={job.last_error}>{job.last_error}</span>
						{/if}
					</div>
				</button>
			{/each}
		</div>
	{/if}

	<!-- Job detail modal -->
	{#if selectedJob}
		<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4" onclick={(e) => { if (e.target === e.currentTarget) selectedJob = null; }}>
			<div class="bg-gray-800 rounded-lg border border-gray-700 max-w-2xl w-full max-h-[80vh] flex flex-col">
				<!-- Modal header -->
				<div class="flex items-center justify-between gap-3 p-4 border-b border-gray-700 shrink-0">
					<div>
						<div class="flex items-center gap-2">
							<span class="w-2 h-2 rounded-full {statusDot(selectedJob.status)} inline-block"></span>
							<h3 class="text-white font-medium">{selectedJob.name}</h3>
						</div>
						<div class="text-gray-500 text-xs mt-1">{selectedJob.id} · {selectedJob.queue}</div>
					</div>
					<button
						onclick={() => selectedJob = null}
						class="text-gray-400 hover:text-white text-lg leading-none px-1"
					>&times;</button>
				</div>

				<!-- Modal body -->
				<div class="p-4 space-y-4 overflow-y-auto flex-1">
					<!-- Stats -->
					<div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
						<div class="bg-gray-750 rounded p-3">
							<div class="text-gray-500 text-xs">Runs</div>
							<div class="text-white font-medium">{selectedJob.run_count}</div>
						</div>
						<div class="bg-gray-750 rounded p-3">
							<div class="text-gray-500 text-xs">Succeeded</div>
							<div class="text-green-400 font-medium">{selectedJob.success_count}</div>
						</div>
						<div class="bg-gray-750 rounded p-3">
							<div class="text-gray-500 text-xs">Failed</div>
							<div class="text-red-400 font-medium">{selectedJob.failure_count}</div>
						</div>
						<div class="bg-gray-750 rounded p-3">
							<div class="text-gray-500 text-xs">Avg Duration</div>
							<div class="text-white font-medium">{formatDuration(selectedJob.avg_duration_ms)}</div>
						</div>
					</div>

					<!-- Timing -->
					<div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-sm">
						<div class="bg-gray-750 rounded p-3">
							<div class="text-gray-500 text-xs">Last Run</div>
							<div class="text-gray-300">{formatTimestamp(selectedJob.last_run_at)}</div>
						</div>
						<div class="bg-gray-750 rounded p-3">
							<div class="text-gray-500 text-xs">Next Run</div>
							<div class="text-gray-300">{formatTimestamp(selectedJob.next_run_at)}</div>
						</div>
					</div>

					<!-- Labels -->
					{#if selectedJob.labels.length > 0}
						<div class="flex flex-wrap gap-1.5">
							{#each selectedJob.labels as label}
								<span class="px-2 py-0.5 text-xs rounded bg-gray-700 text-gray-400">{label}</span>
							{/each}
						</div>
					{/if}

					<!-- Last error -->
					{#if selectedJob.last_error}
						<div class="bg-red-900/20 border border-red-800/40 rounded p-3">
							<div class="text-red-400 text-xs font-medium mb-1">Last Error — {formatTimestamp(selectedJob.last_error_at)}</div>
							<pre class="text-red-300/80 text-xs whitespace-pre-wrap break-all">{selectedJob.last_error}</pre>
						</div>
					{/if}

					<!-- Recent logs -->
					<div>
						<h4 class="text-gray-300 text-sm font-medium mb-2">Recent Runs ({selectedJob.recent_logs.length})</h4>
						{#if selectedJob.recent_logs.length === 0}
							<div class="text-gray-600 text-xs py-4 text-center">No log entries yet</div>
						{:else}
							<div class="space-y-1.5 max-h-48 overflow-y-auto">
								{#each selectedJob.recent_logs as log}
									<div class="flex items-center gap-2 text-xs py-1.5 px-2 rounded {log.outcome === 'failure' ? 'bg-red-900/10' : log.outcome === 'running' ? 'bg-blue-900/10' : 'bg-gray-750'}">
										<span class="w-1.5 h-1.5 rounded-full shrink-0 {log.outcome === 'success' ? 'bg-green-400' : log.outcome === 'failure' ? 'bg-red-400' : 'bg-blue-400'}"></span>
										<span class="text-gray-400 w-16 shrink-0">{log.outcome}</span>
										<span class="text-gray-500 flex-1 truncate">{formatTimestamp(log.started_at)}</span>
										<span class="text-gray-600 shrink-0">{formatDuration(log.duration_ms)}</span>
									</div>
								{/each}
							</div>
						{/if}
					</div>
				</div>

				<!-- Modal footer -->
				{#if selectedJob}
					{@const job = selectedJob}
					<div class="flex items-center justify-between gap-3 p-4 border-t border-gray-700 shrink-0">
						<div class="text-sm">
							<span class="px-2 py-0.5 text-xs rounded border {statusBadge(job.status)}">{job.status}</span>
							{#if job.paused}
								<span class="text-amber-400 text-xs ml-2">Paused</span>
							{/if}
						</div>
						<div class="flex gap-2">
							{#if job.status === 'failed'}
								<button
									onclick={() => retryJob(job.id)}
									disabled={actionLoading === job.id}
									class="px-3 py-1.5 text-xs rounded bg-amber-700/40 text-amber-300 hover:bg-amber-700/60 disabled:opacity-50 transition-colors"
								>{actionLoading === job.id ? '…' : 'Retry'}</button>
							{/if}
							{#if job.paused}
								<button
									onclick={() => resumeJob(job.id)}
									disabled={actionLoading === job.id}
									class="px-3 py-1.5 text-xs rounded bg-green-700/40 text-green-300 hover:bg-green-700/60 disabled:opacity-50 transition-colors"
								>{actionLoading === job.id ? '…' : 'Resume'}</button>
							{:else if job.status !== 'failed'}
								<button
									onclick={() => pauseJob(job.id)}
									disabled={actionLoading === job.id}
									class="px-3 py-1.5 text-xs rounded bg-amber-700/40 text-amber-300 hover:bg-amber-700/60 disabled:opacity-50 transition-colors"
								>{actionLoading === job.id ? '…' : 'Pause'}</button>
							{/if}
							<button
								onclick={() => selectedJob = null}
								class="px-3 py-1.5 text-xs rounded bg-gray-700 text-gray-200 hover:bg-gray-600 transition-colors"
							>Close</button>
						</div>
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>
