<script lang="ts">
	/**
	 * @file Email Verification page
	 * @purpose Confirm email ownership via token from registration/verification email
	 * @depends page.url.searchParams.token
	 * @version 1.0.0
	 */
	import { page } from '$app/stores';

	let token = $state('');
	let status: 'loading' | 'success' | 'error' = $state('loading');
	let message = $state('Verifying your email…');

	$effect.pre(() => {
		token = $page.url.searchParams.get('token') || '';
	});

	async function verifyEmail() {
		if (!token) {
			status = 'error';
			message = 'No verification token found. Use the link from your email.';
			return;
		}

		try {
			const response = await fetch(`/api/v1/auth/verify-email?token=${encodeURIComponent(token)}`);

			if (response.ok) {
				status = 'success';
				message = 'Your email has been verified! You can now log in and use all features.';
			} else {
				const data = await response.json().catch(() => ({}));
				status = 'error';
				message = data.message || 'This verification link is invalid or has expired. Please request a new one from your settings.';
			}
		} catch (e: any) {
			status = 'error';
			message = 'Something went wrong. Please try again or request a new verification email.';
		}
	}

	// Run verification when token is ready
	$effect(() => {
		if (token) verifyEmail();
	});
</script>

<svelte:head>
	<title>Email Verification — OpenCorde</title>
</svelte:head>

<div class="flex items-start justify-center min-h-screen bg-gray-900 px-4 py-6 sm:items-center sm:py-8">
	<div class="w-full max-w-md p-6 sm:p-8 bg-gray-800 rounded-xl shadow-xl text-center">
		{#if status === 'loading'}
			<div class="space-y-4">
				<div class="mx-auto w-12 h-12 border-4 border-gray-600 border-t-indigo-500 rounded-full animate-spin"></div>
				<h1 class="text-xl font-bold text-white">Verifying your email</h1>
				<p class="text-gray-400 text-sm">Please wait a moment…</p>
			</div>
		{:else if status === 'success'}
			<div class="space-y-4">
				<div class="mx-auto w-12 h-12 bg-green-900/30 border border-green-700/40 rounded-full flex items-center justify-center">
					<svg class="w-6 h-6 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
					</svg>
				</div>
				<h1 class="text-xl font-bold text-white">Email Verified</h1>
				<p class="text-gray-400 text-sm">{message}</p>
				<a
					href="/login"
					class="inline-block mt-4 px-6 py-2.5 bg-indigo-600 hover:bg-indigo-500 text-white font-medium rounded transition-colors"
				>
					Continue to Login
				</a>
			</div>
		{:else}
			<div class="space-y-4">
				<div class="mx-auto w-12 h-12 bg-red-900/30 border border-red-700/40 rounded-full flex items-center justify-center">
					<svg class="w-6 h-6 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
					</svg>
				</div>
				<h1 class="text-xl font-bold text-white">Verification Failed</h1>
				<p class="text-gray-400 text-sm">{message}</p>
				<div class="flex gap-3 justify-center mt-4">
					<a
						href="/login"
						class="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white text-sm font-medium rounded transition-colors"
					>
						Back to Login
					</a>
				</div>
			</div>
		{/if}
	</div>
</div>
