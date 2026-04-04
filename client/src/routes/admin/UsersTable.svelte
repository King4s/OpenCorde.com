<script lang="ts">
	/**
	 * @file Users table component
	 * @purpose Display and manage admin users list
	 */
	import type { AdminUserRow } from '$lib/api/types';
	import api from '$lib/api/client';

	interface Props {
		users: AdminUserRow[];
		loading: boolean;
		totalUsers: number;
		offset: number;
		limit: number;
		onDelete: (type: 'user' | 'server', id: string, name: string) => void;
		onPrevPage: () => void;
		onNextPage: () => void;
	}

	const { users, loading, totalUsers, offset, limit, onDelete, onPrevPage, onNextPage }: Props =
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
		<div class="p-8 text-center text-gray-400">Loading users...</div>
	{:else if users.length === 0}
		<div class="p-8 text-center text-gray-400">No users found</div>
	{:else}
		<div class="overflow-x-auto">
			<table class="w-full text-sm">
				<thead class="bg-gray-900">
					<tr class="border-b border-gray-700">
						<th class="px-4 py-3 text-left text-gray-400 font-semibold">Username</th>
						<th class="px-4 py-3 text-left text-gray-400 font-semibold">Email</th>
						<th class="px-4 py-3 text-left text-gray-400 font-semibold">Created</th>
						<th class="px-4 py-3 text-center text-gray-400 font-semibold">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each users as user (user.id)}
						<tr class="border-b border-gray-700 hover:bg-gray-700/50 transition">
							<td class="px-4 py-3 text-white font-medium">{user.username}</td>
							<td class="px-4 py-3 text-gray-300">{user.email}</td>
							<td class="px-4 py-3 text-gray-400 text-xs">{formatDate(user.created_at)}</td>
							<td class="px-4 py-3 text-center">
								<button
									onclick={() => onDelete('user', user.id, user.username)}
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
				Showing {offset + 1}-{Math.min(offset + limit, totalUsers)} of {totalUsers}
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
					disabled={offset + limit >= totalUsers}
					class="px-3 py-1 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 text-gray-300 text-xs rounded transition-colors"
				>
					Next
				</button>
			</div>
		</div>
	{/if}
</div>
