<script lang="ts">
	/**
	 * @file Member context menu
	 * @purpose Right-click actions for server members (kick, ban, timeout, profile)
	 * @depends moderation store, svelte
	 */
	import { kickUser, banUser, timeoutUser } from '$lib/stores/moderation';

	interface Props {
		userId: string;
		username: string;
		spaceId: string;
		isOwner: boolean;
		x: number;
		y: number;
		onClose: () => void;
	}

	let { userId, username, spaceId, isOwner, x, y, onClose }: Props = $props();

	let showBanReason = $state(false);
	let banReason = $state('');
	let showTimeout = $state(false);
	let timeoutHours = $state(1);
	let error = $state('');

	async function handleKick() {
		if (confirm(`Kick ${username}?`)) {
			try {
				await kickUser(spaceId, userId);
				onClose();
			} catch (err) {
				error = `Failed to kick: ${(err as any)?.message || 'Unknown error'}`;
			}
		}
	}

	async function handleBan() {
		try {
			await banUser(spaceId, userId, banReason || undefined);
			onClose();
		} catch (err) {
			error = `Failed to ban: ${(err as any)?.message || 'Unknown error'}`;
		}
	}

	async function handleTimeout() {
		try {
			await timeoutUser(spaceId, userId, timeoutHours * 3600);
			onClose();
		} catch (err) {
			error = `Failed to timeout: ${(err as any)?.message || 'Unknown error'}`;
		}
	}

	function handleNavigateProfile() {
		// Placeholder: navigate to user profile
		window.location.href = `/users/${userId}`;
		onClose();
	}
</script>

<!-- Click-outside backdrop -->
<div
	class="fixed inset-0 z-40"
	role="button"
	tabindex="-1"
	aria-label="Close context menu"
	onclick={onClose}
	onkeydown={(e) => e.key === 'Escape' && onClose()}
></div>

<!-- Context menu -->
<div
	class="fixed z-50 bg-gray-900 border border-gray-700 rounded-lg shadow-xl py-1 min-w-48 max-w-60"
	style="left: {x}px; top: {y}px"
>
	<div class="px-3 py-1.5 text-xs text-gray-400 font-semibold uppercase border-b border-gray-700">
		{username}
	</div>

	{#if error}
		<div class="px-3 py-1.5 text-xs text-gray-400 border-b border-gray-700">
			{error}
		</div>
	{/if}

	{#if !showBanReason && !showTimeout}
		<button
			class="w-full text-left px-3 py-1.5 text-sm text-gray-300 hover:bg-gray-800 transition-colors"
			onclick={handleNavigateProfile}
		>
			View Profile
		</button>

		{#if isOwner}
			<div class="border-t border-gray-700 my-1"></div>

			<button
				class="w-full text-left px-3 py-1.5 text-sm text-gray-400 hover:bg-gray-800 transition-colors"
				onclick={() => {
					showTimeout = true;
					error = '';
				}}
			>
				Timeout
			</button>

			<button
				class="w-full text-left px-3 py-1.5 text-sm text-gray-400 hover:bg-gray-800 transition-colors"
				onclick={handleKick}
			>
				Kick from Space
			</button>

			<button
				class="w-full text-left px-3 py-1.5 text-sm text-gray-400 hover:bg-gray-800 transition-colors"
				onclick={() => {
					showBanReason = true;
					error = '';
				}}
			>
				Ban from Space
			</button>
		{/if}
	{:else if showBanReason}
		<div class="px-3 py-2 border-t border-gray-700">
			<label for="ban-reason-input" class="block text-xs text-gray-400 mb-1">Ban reason (optional)</label>
			<input
				id="ban-reason-input"
				type="text"
				bind:value={banReason}
				placeholder="Enter reason"
				class="w-full bg-gray-800 text-white text-sm rounded px-2 py-1.5 mb-2 outline-none border border-gray-700 focus:border-gray-600"
			/>
			<div class="flex gap-2">
				<button
					onclick={handleBan}
					class="flex-1 bg-gray-700 hover:bg-gray-600 text-white text-xs font-medium py-1.5 rounded transition-colors"
				>
					Ban
				</button>
				<button
					onclick={() => {
						showBanReason = false;
						banReason = '';
						error = '';
					}}
					class="flex-1 bg-gray-700 hover:bg-gray-600 text-white text-xs font-medium py-1.5 rounded transition-colors"
				>
					Cancel
				</button>
			</div>
		</div>
	{:else if showTimeout}
		<div class="px-3 py-2 border-t border-gray-700">
			<label for="timeout-select" class="block text-xs text-gray-400 mb-1">Timeout duration</label>
			<select
				id="timeout-select"
				bind:value={timeoutHours}
				class="w-full bg-gray-800 text-white text-sm rounded px-2 py-1.5 mb-2 outline-none border border-gray-700 focus:border-gray-600"
			>
				<option value={0.25}>15 minutes</option>
				<option value={1}>1 hour</option>
				<option value={4}>4 hours</option>
				<option value={24}>1 day</option>
				<option value={168}>1 week</option>
			</select>
			<div class="flex gap-2">
				<button
					onclick={handleTimeout}
					class="flex-1 bg-gray-700 hover:bg-gray-600 text-white text-xs font-medium py-1.5 rounded transition-colors"
				>
					Timeout
				</button>
				<button
					onclick={() => {
						showTimeout = false;
						timeoutHours = 1;
						error = '';
					}}
					class="flex-1 bg-gray-700 hover:bg-gray-600 text-white text-xs font-medium py-1.5 rounded transition-colors"
				>
					Cancel
				</button>
			</div>
		</div>
	{/if}
</div>
