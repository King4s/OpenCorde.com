<script lang="ts">
	/**
	 * @file Password Reset page
	 * @purpose Complete password reset with token from email
	 * @depends page.url.searchParams.token
	 * @version 1.0.0
	 */
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';

	let token = $state('');
	let newPassword = $state('');
	let confirmPassword = $state('');
	let error = $state('');
	let success = $state('');
	let loading = $state(false);

	$effect.pre(() => {
		token = $page.url.searchParams.get('token') || '';
	});

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';
		success = '';
		loading = true;

		if (!token) {
			error = 'Reset token is missing. Please use the link from your email.';
			loading = false;
			return;
		}

		if (newPassword.length < 8) {
			error = 'Password must be at least 8 characters.';
			loading = false;
			return;
		}

		if (newPassword !== confirmPassword) {
			error = 'Passwords do not match.';
			loading = false;
			return;
		}

		try {
			const response = await fetch('/api/v1/auth/reset-password', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					token,
					new_password: newPassword
				})
			});

			if (!response.ok) {
				const data = await response.json();
				throw new Error(data.message || 'Failed to reset password');
			}

			success = 'Password reset successfully! Redirecting to login...';
			setTimeout(() => {
				window.location.href = '/login';
			}, 2000);
		} catch (e: any) {
			error = e.message || 'Failed to reset password';
		} finally {
			loading = false;
		}
	}
</script>

<div class="flex items-center justify-center min-h-screen bg-gray-900">
	<div class="w-full max-w-md p-8 bg-gray-800 rounded-lg shadow-xl">
		<h1 class="text-2xl font-bold text-white mb-2">Reset Password</h1>
		<p class="text-gray-400 mb-6">Enter your new password below</p>

		{#if error}
			<div class="bg-gray-900/50 text-gray-300 p-3 rounded mb-4 text-sm">{error}</div>
		{/if}

		{#if success}
			<div class="bg-gray-900/50 text-gray-300 p-3 rounded mb-4 text-sm">{success}</div>
		{/if}

		{#if !token}
			<div class="bg-gray-900/50 text-gray-300 p-4 rounded text-sm">
				<p class="font-bold mb-2">Reset token not found</p>
				<p>Please use the reset link from your email.</p>
				<a href="/login" class="text-gray-400 hover:underline mt-2 inline-block">
					Back to Login
				</a>
			</div>
		{:else}
			<form onsubmit={handleSubmit} class="space-y-4">
				<div>
					<label for="password" class="block text-sm font-medium text-gray-300 mb-1"
						>New Password</label
					>
					<input
						id="password"
						type="password"
						bind:value={newPassword}
						required
						class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-gray-500"
						placeholder="••••••••"
					/>
					<p class="text-gray-500 text-xs mt-1">Minimum 8 characters</p>
				</div>

				<div>
					<label for="confirm" class="block text-sm font-medium text-gray-300 mb-1"
						>Confirm Password</label
					>
					<input
						id="confirm"
						type="password"
						bind:value={confirmPassword}
						required
						class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-gray-500"
						placeholder="••••••••"
					/>
				</div>

				<button
					type="submit"
					disabled={loading}
					class="w-full py-2 bg-gray-600 hover:bg-gray-700 disabled:opacity-50 text-white font-medium rounded transition-colors"
				>
					{loading ? 'Resetting...' : 'Reset Password'}
				</button>
			</form>

			<p class="text-gray-400 text-sm mt-4 text-center">
				<a href="/login" class="text-gray-400 hover:underline">Back to Login</a>
			</p>
		{/if}
	</div>
</div>
