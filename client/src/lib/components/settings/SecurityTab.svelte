<script lang="ts">
	/**
	 * @file SecurityTab — account security settings: 2FA, sessions, authorized apps
	 * @purpose Central security hub linking 2FA setup, session management, and app authorization
	 * @depends $lib/components/settings/TwoFactorSetup, SessionsPanel, AuthorizedAppsPanel
	 */
	import { currentUser } from '$lib/stores/auth';
	import TwoFactorSetup from '$lib/components/settings/TwoFactorSetup.svelte';
	import SessionsPanel from '$lib/components/settings/SessionsPanel.svelte';
	import AuthorizedAppsPanel from '$lib/components/settings/AuthorizedAppsPanel.svelte';

	let totpEnabled = $state(false);

	$effect(() => {
		if ($currentUser) {
			totpEnabled = $currentUser.totp_enabled;
		}
	});
</script>

<div class="space-y-4">
	<!-- 2FA -->
	<TwoFactorSetup enabled={totpEnabled} onchange={(val) => { totpEnabled = val; }} />

	<!-- Sessions & Devices (stub — sibling card will fill in) -->
	<SessionsPanel />

	<!-- Authorized Apps (stub — sibling card will fill in) -->
	<AuthorizedAppsPanel />

	<!-- Password change (placeholder) -->
	<div class="bg-gray-800 rounded-lg p-3 sm:p-4">
		<h2 class="text-sm font-semibold text-gray-400 uppercase mb-3">Password</h2>
		<p class="text-xs text-gray-500">
			Password change is available from the login page. Use the "Forgot password" link
			on the sign-in screen to reset your password.
		</p>
	</div>
</div>
