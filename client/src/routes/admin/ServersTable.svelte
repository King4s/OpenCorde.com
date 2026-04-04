<script lang="ts">
	/**
	 * @file Servers table component
	 * @purpose Display and manage admin instances list
	 */
	import type { AdminServerRow } from '$lib/api/types';

	interface Props {
		servers: AdminServerRow[];
		loading: boolean;
		totalServers: number;
		offset: number;
		limit: number;
		onDelete: (type: 'user' | 'server', id: string, name: string) => void;
		onPrevPage: () => void;
		onNextPage: () => void;
	}

	const { servers, loading, totalServers, offset, limit, onDelete, onPrevPage, onNextPage }: Props =
		$props();

	function formatDate(date: string) {
		return new Date(date).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<div class="bg-gray-800 rounded-lg overflow-hidden">
	{#if loading}
		<div class="p-8 text-center text-gray-400">Loading instances...</div>
	{:else if servers.length === 0}
		<div class="p-8 text-center text-gray-400">No instances found</div>
	{:else}
		<div class="overflow-x-auto">
			<table class="w-full text-sm">
				<thead class="bg-gray-900">
					<tr class="border-b border-gray-700">
						<th class="px-4 py-3 text-left text-gray-400 font-semibold">Space Name</th>
						<th class="px-4 py-3 text-left text-gray-400 font-semibold">Owner ID</th>
						<th class="px-4 py-3 text-center text-gray-400 font-semibold">Members</th>
						<th class="px-4 py-3 text-left text-gray-400 font-semibold">Created</th>
						<th class="px-4 py-3 text-center text-gray-400 font-semibold">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each servers as server (server.id)}
						<tr class="border-b border-gray-700 hover:bg-gray-700/50 transition">
							<td class="px-4 py-3 text-white font-medium">{server.name}</td>
							<td class="px-4 py-3 text-gray-300 text-xs">{server.owner_id}</td>
							<td class="px-4 py-3 text-center text-gray-300">{server.member_count}</td>
							<td class="px-4 py-3 text-gray-400 text-xs">{formatDate(server.created_at)}</td>
							<td class="px-4 py-3 text-center">
								<button
									onclick={() => onDelete('server', server.id, server.name)}
									class="px-3 py-1 bg-gray-600 hover:bg-gray-700 text-white text-xs rounded transition-colors"
								>
									Delete
								</button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<!-- Pagination Controls -->
		<div class="flex items-center justify-between px-4 py-3 bg-gray-900 border-t border-gray-700">
			<div class="text-xs text-gray-400">
				Showing {offset + 1}-{Math.min(offset + limit, totalServers)} of {totalServers}
			</div>
			<div class="flex gap-2">
				<button
					onclick={onPrevPage}
					disabled={offset === 0}
					class="px-3 py-1 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 text-gray-300 text-xs rounded transition-colors"
				>
					Previous
				</button>
				<button
					onclick={onNextPage}
					disabled={offset + limit >= totalServers}
					class="px-3 py-1 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 text-gray-300 text-xs rounded transition-colors"
				>
					Next
				</button>
			</div>
		</div>
	{/if}
</div>
