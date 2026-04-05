<script lang="ts">
	/**
	 * @file Register page
	 * @purpose User registration with email, username, and password, Steam OpenID option
	 * @depends stores/auth
	 * @version 2.1.0
	 */
	import { onMount } from 'svelte';
	import { establishSession, register } from '$lib/stores/auth';

	let email = $state('');
	let username = $state('');
	let password = $state('');
	let passwordConfirm = $state('');
	let error = $state('');
	let loading = $state(false);
	let inviteCode = $state('');

	// Handle Steam callback
	onMount(async () => {
		const params = new URLSearchParams(window.location.search);
		const accessToken = params.get('accessToken');
		const refreshToken = params.get('refreshToken');

		if (accessToken && refreshToken) {
			try {
				localStorage.setItem('opencorde_refresh_token', refreshToken);
				await establishSession(accessToken);
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
			await register(username, email, password, inviteCode || undefined);
			window.location.href = '/servers';
		} catch (e: any) {
			error = e.message || 'Registration failed';
		} finally {
			loading = false;
		}
	}
</script>

<div class="flex items-start justify-center min-h-screen bg-gray-900 px-4 py-6 sm:items-center sm:py-8">
	<div class="w-full max-w-md p-6 sm:p-8 bg-gray-800 rounded-xl shadow-xl">
		<h1 class="text-2xl font-bold text-white mb-1">Create an account</h1>
		<p class="text-gray-400 mb-6 text-sm">Join OpenCorde — free and open source</p>

		{#if error}
			<div role="alert" class="bg-red-900/30 border border-red-700/40 text-red-300 p-3 rounded mb-4 text-sm">{error}</div>
		{/if}

		<form onsubmit={handleSubmit} class="space-y-4">
			<div>
				<label for="email" class="block text-sm font-medium text-gray-300 mb-1">
					Email <span class="text-gray-500 font-normal">(required)</span>
				</label>
				<input
					id="email"
					type="email"
					bind:value={email}
					required
					autocomplete="email"
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500/50"
					placeholder="you@example.com"
				/>
			</div>

			<div>
				<label for="username" class="block text-sm font-medium text-gray-300 mb-1">
					Username <span class="text-gray-500 font-normal">(required)</span>
				</label>
				<input
					id="username"
					type="text"
					bind:value={username}
					required
					autocomplete="username"
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500/50"
					placeholder="your_username"
				/>
				<p class="text-gray-500 text-xs mt-1">3–32 characters, letters, numbers, and underscores only.</p>
			</div>

			<div>
				<label for="password" class="block text-sm font-medium text-gray-300 mb-1">
					Password <span class="text-gray-500 font-normal">(required)</span>
				</label>
				<input
					id="password"
					type="password"
					bind:value={password}
					required
					autocomplete="new-password"
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500/50"
					placeholder="••••••••"
				/>
				<p class="text-gray-500 text-xs mt-1">At least 6 characters.</p>
			</div>

			<div>
				<label for="confirm" class="block text-sm font-medium text-gray-300 mb-1">
					Confirm Password <span class="text-gray-500 font-normal">(required)</span>
				</label>
				<input
					id="confirm"
					type="password"
					bind:value={passwordConfirm}
					required
					autocomplete="new-password"
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500/50"
					placeholder="••••••••"
				/>
			</div>

			<div>
				<label for="invite" class="block text-sm font-medium text-gray-300 mb-1">
					Invite Code <span class="text-gray-500 font-normal">(optional — only needed if registration requires one)</span>
				</label>
				<input
					id="invite"
					type="text"
					bind:value={inviteCode}
					autocomplete="off"
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500/50"
					placeholder="Leave blank if not required"
				/>
			</div>

			<button
				type="submit"
				disabled={loading}
				class="w-full py-2.5 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed text-white font-medium rounded transition-colors"
			>
				{loading ? 'Creating account…' : 'Create Account'}
			</button>
		</form>

		<p class="text-gray-400 text-sm mt-4 text-center">
			Already have an account?
			<a href="/login" class="text-indigo-400 hover:text-indigo-300 underline">Log in</a>
		</p>

		<!-- Divider -->
		<div class="flex items-center my-6">
			<div class="flex-1 border-t border-gray-700"></div>
			<span class="px-3 text-gray-500 text-xs">or continue with</span>
			<div class="flex-1 border-t border-gray-700"></div>
		</div>

		<!-- Steam Sign Up -->
		<a
			href="/api/v1/auth/steam"
			class="w-full py-2.5 bg-gray-700 hover:bg-gray-600 text-white font-medium rounded transition-colors flex items-center justify-center gap-2"
			aria-label="Sign up using your Steam account"
		>
			<svg class="w-5 h-5" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
				<path
					d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm3.5-9c.83 0 1.5-.67 1.5-1.5S16.33 8 15.5 8 14 8.67 14 9.5s.67 1.5 1.5 1.5zm-7 0c.83 0 1.5-.67 1.5-1.5S9.33 8 8.5 8 7 8.67 7 9.5 7.67 11 8.5 11zm3.5 6.5c2.33 0 4.31-1.46 5.11-3.5H6.89c.8 2.04 2.78 3.5 5.11 3.5z"
				/>
			</svg>
			Sign up with Steam
		</a>
	</div>
</div>
