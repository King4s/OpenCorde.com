<script lang="ts">
	/**
	 * @file NotificationsTab — push notification toggle and preferences
	 * @purpose Enable/disable push notifications for browser
	 * @depends $lib/stores/pushNotifications
	 */
	import { notificationsEnabled, registerPushToken, unregisterPushToken } from '$lib/stores/pushNotifications';

	let pushLoading = $state(false);
	let pushError = $state('');

	async function handlePushToggle() {
		pushLoading = true;
		pushError = '';
		try {
			if ($notificationsEnabled) {
				await unregisterPushToken();
			} else {
				await registerPushToken();
			}
		} catch (e: any) {
			pushError = e.message ?? 'Notification toggle failed';
		} finally {
			pushLoading = false;
		}
	}
</script>

<div class="bg-gray-800 rounded-lg p-4 space-y-4">
	<h2 class="text-sm font-semibold text-gray-400 uppercase">Notifications</h2>
	{#if pushError}
		<div class="px-3 py-2 bg-gray-900/40 border border-gray-700/50 rounded text-gray-300 text-sm">{pushError}</div>
	{/if}
	<div class="flex items-center justify-between">
		<div>
			<p class="text-sm text-gray-300">Push Notifications</p>
			<p class="text-xs text-gray-500 mt-0.5">Receive alerts for mentions even when the tab is in the background.</p>
		</div>
		<button
			onclick={handlePushToggle}
			disabled={pushLoading}
			class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors disabled:opacity-50
				{$notificationsEnabled ? 'bg-gray-600' : 'bg-gray-600'}"
			aria-pressed={$notificationsEnabled}
			aria-label="Toggle push notifications"
		>
			<span class="inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform
				{$notificationsEnabled ? 'translate-x-6' : 'translate-x-1'}">
			</span>
		</button>
	</div>
</div>
