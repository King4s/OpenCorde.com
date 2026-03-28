<script lang="ts">
	/**
	 * @file Email verification page
	 * @purpose Handle the verify-email?token=... link sent on registration
	 * @depends api/client
	 */
	import { browser } from '$app/environment';
	import { page } from '$app/stores';
	import api from '$lib/api/client';

	type Status = 'verifying' | 'success' | 'error';

	let status = $state<Status>('verifying');
	let errorMessage = $state('');

	if (browser) {
		const token = $page.url.searchParams.get('token');
		if (!token) {
			status = 'error';
			errorMessage = 'No verification token provided.';
		} else {
			api.get(`/auth/verify-email?token=${encodeURIComponent(token)}`)
				.then(() => {
					status = 'success';
				})
				.catch((e: any) => {
					status = 'error';
					errorMessage = e.message || 'Verification failed. The link may have expired.';
				});
		}
	}
</script>

<div class="min-h-screen bg-gray-900 flex items-center justify-center px-4">
	<div class="max-w-md w-full bg-gray-800 rounded-xl p-8 shadow-xl text-center">
		<!-- Logo -->
		<div class="w-16 h-16 bg-indigo-600 rounded-2xl flex items-center justify-center mx-auto mb-6">
			<span class="text-white text-3xl font-bold">O</span>
		</div>

		{#if status === 'verifying'}
			<h1 class="text-2xl font-bold text-white mb-3">Verifying your email...</h1>
			<div class="flex justify-center">
				<div class="w-6 h-6 border-2 border-indigo-500 border-t-transparent rounded-full animate-spin"></div>
			</div>

		{:else if status === 'success'}
			<div class="w-14 h-14 bg-green-600/20 rounded-full flex items-center justify-center mx-auto mb-4">
				<svg class="w-7 h-7 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
				</svg>
			</div>
			<h1 class="text-2xl font-bold text-white mb-2">Email Verified!</h1>
			<p class="text-gray-400 mb-6">Your email address has been confirmed. You can now use all features.</p>
			<a
				href="/login"
				class="inline-block w-full px-4 py-2.5 bg-indigo-600 hover:bg-indigo-500 text-white font-semibold rounded-lg transition-colors"
			>
				Go to Login
			</a>

		{:else}
			<div class="w-14 h-14 bg-red-600/20 rounded-full flex items-center justify-center mx-auto mb-4">
				<svg class="w-7 h-7 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
				</svg>
			</div>
			<h1 class="text-2xl font-bold text-white mb-2">Verification Failed</h1>
			<p class="text-gray-400 mb-2">{errorMessage}</p>
			<p class="text-gray-500 text-sm mb-6">Verification links expire after 24 hours. Register again to get a new link.</p>
			<a
				href="/register"
				class="inline-block w-full px-4 py-2.5 bg-indigo-600 hover:bg-indigo-500 text-white font-semibold rounded-lg transition-colors"
			>
				Back to Register
			</a>
		{/if}
	</div>
</div>
