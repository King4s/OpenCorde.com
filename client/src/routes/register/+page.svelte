<script lang="ts">
	/**
	 * @file Register page
	 * @purpose User registration with email, username, and password, Steam OpenID option
	 * @depends stores/auth
	 * @version 2.0.0
	 */
	import { onMount } from 'svelte';
	import { register } from '$lib/stores/auth';

	let email = $state('');
	let username = $state('');
	let password = $state('');
	let passwordConfirm = $state('');
	let error = $state('');
	let loading = $state(false);

	// Handle Steam callback
	onMount(() => {
		const params = new URLSearchParams(window.location.search);
		const accessToken = params.get('access_token');
		const refreshToken = params.get('refresh_token');

		if (accessToken && refreshToken) {
			try {
				localStorage.setItem('access_token', accessToken);
				localStorage.setItem('refresh_token', refreshToken);
				window.location.href = '/servers';
			} catch (e) {
				error = 'Failed to store tokens';
			}
		}
	});

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';

		if (password !== passwordConfirm) {
			error = 'Passwords do not match';
			return;
		}

		if (password.length < 6) {
			error = 'Password must be at least 6 characters';
			return;
		}

		loading = true;
		try {
			await register(username, email, password);
			window.location.href = '/servers';
		} catch (e: any) {
			error = e.message || 'Registration failed';
		} finally {
			loading = false;
		}
	}
</script>

<div class="flex items-center justify-center min-h-screen bg-gray-900">
	<div class="w-full max-w-md p-8 bg-gray-800 rounded-lg shadow-xl">
		<h1 class="text-2xl font-bold text-white mb-2">Create an account</h1>
		<p class="text-gray-400 mb-6">Join OpenCorde</p>

		{#if error}
			<div class="bg-red-900/50 text-red-300 p-3 rounded mb-4 text-sm">{error}</div>
		{/if}

		<form onsubmit={handleSubmit} class="space-y-4">
			<div>
				<label for="email" class="block text-sm font-medium text-gray-300 mb-1"
					>Email</label
				>
				<input
					id="email"
					type="email"
					bind:value={email}
					required
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500"
					placeholder="you@example.com"
				/>
			</div>

			<div>
				<label for="username" class="block text-sm font-medium text-gray-300 mb-1"
					>Username</label
				>
				<input
					id="username"
					type="text"
					bind:value={username}
					required
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500"
					placeholder="username"
				/>
			</div>

			<div>
				<label for="password" class="block text-sm font-medium text-gray-300 mb-1"
					>Password</label
				>
				<input
					id="password"
					type="password"
					bind:value={password}
					required
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500"
					placeholder="••••••••"
				/>
			</div>

			<div>
				<label for="confirm" class="block text-sm font-medium text-gray-300 mb-1"
					>Confirm Password</label
				>
				<input
					id="confirm"
					type="password"
					bind:value={passwordConfirm}
					required
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500"
					placeholder="••••••••"
				/>
			</div>

			<button
				type="submit"
				disabled={loading}
				class="w-full py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white font-medium rounded transition-colors"
			>
				{loading ? 'Creating account...' : 'Register'}
			</button>
		</form>

		<p class="text-gray-400 text-sm mt-4 text-center">
			Already have an account? <a href="/login" class="text-indigo-400 hover:underline"
				>Log in</a
			>
		</p>

		<!-- Divider -->
		<div class="flex items-center my-6">
			<div class="flex-1 border-t border-gray-700"></div>
			<span class="px-3 text-gray-500 text-xs">or</span>
			<div class="flex-1 border-t border-gray-700"></div>
		</div>

		<!-- Steam Login Button -->
		<a
			href="/api/v1/auth/steam"
			class="w-full py-2 bg-gray-700 hover:bg-gray-600 text-white font-medium rounded transition-colors flex items-center justify-center gap-2 block"
		>
			<svg class="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
				<path
					d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm3.5-9c.83 0 1.5-.67 1.5-1.5S16.33 8 15.5 8 14 8.67 14 9.5s.67 1.5 1.5 1.5zm-7 0c.83 0 1.5-.67 1.5-1.5S9.33 8 8.5 8 7 8.67 7 9.5 7.67 11 8.5 11zm3.5 6.5c2.33 0 4.31-1.46 5.11-3.5H6.89c.8 2.04 2.78 3.5 5.11 3.5z"
				/>
			</svg>
			Sign up with Steam
		</a>
	</div>
</div>
