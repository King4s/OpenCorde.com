<script lang="ts">
	/**
	 * @file Message permalink redirect page
	 * @purpose Redirects /servers/:serverId/channels/:channelId/:messageId
	 *          to /servers/:serverId/channels/:channelId?msg=:messageId
	 */
	import { browser } from '$app/environment';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';

	if (browser) {
		const serverId = $page.params.serverId;
		const channelId = $page.params.channelId;
		const messageId = $page.params.messageId;
		goto(`/servers/${serverId}/channels/${channelId}?msg=${messageId}`, { replaceState: true });
	}
</script>

<div class="flex-1 flex items-center justify-center bg-gray-900">
	<p class="text-gray-500 text-sm">Loading message...</p>
</div>
