<script lang="ts">
	/**
	 * @file Login page
	 * @purpose Email + password authentication with forgot password link, Steam OpenID option
	 * @depends stores/auth
	 * @version 3.1.0
	 */
	import { goto } from '$app/navigation';
	import { establishSession, login } from '$lib/stores/auth';
	import { onMount } from 'svelte';

	let email = $state('');
	let password = $state('');
	let error = $state('');
	let loading = $state(false);
	let showForgotPassword = $state(false);
	let showTotp = $state(false);
	let totpCode = $state('');

	// Handle OAuth-style callback from Steam
	onMount(async () => {
		const params = new URLSearchParams(window.location.search);
		const accessToken = params.get('accessToken');
		const refreshToken = params.get('refreshToken');
		const steamError = params.get('error');

		if (steamError === 'steam_failed') {
			error = 'Steam login failed. Please try again.';
			return;
		}

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
		loading = true;
		try {
			await login(email, password, showTotp ? totpCode : undefined);
			window.location.href = '/servers';
		} catch (e: any) {
			if (e.code === 'TWO_FACTOR_REQUIRED') {
				showTotp = true;
				totpCode = '';
				error = '';
			} else {
				error = e.message || 'Login failed';
			}
		} finally {
			loading = false;
		}
	}

	let forgotEmail = $state('');
	let forgotError = $state('');
	let forgotSuccess = $state('');
	let forgotLoading = $state(false);

	async function handleForgotSubmit(e: Event) {
		e.preventDefault();
		forgotError = '';
		forgotSuccess = '';
		forgotLoading = true;

		try {
			const response = await fetch('/api/v1/auth/forgot-password', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ email: forgotEmail })
			});

			if (!response.ok) {
				throw new Error('Failed to request reset');
			}

			forgotSuccess = 'If this email exists, a reset link has been sent.';
			forgotEmail = '';
		} catch (e: any) {
			forgotError = e.message || 'Failed to request reset';
		} finally {
			forgotLoading = false;
		}
	}
</script>

<div class="flex items-start justify-center min-h-screen bg-gray-900 px-4 py-6 sm:items-center sm:py-8">
	<div class="w-full max-w-md p-6 sm:p-8 bg-gray-800 rounded-xl shadow-xl">
		<h1 class="text-2xl font-bold text-white mb-1">Welcome back!</h1>
		<p class="text-gray-400 mb-6 text-sm">Log in to your OpenCorde account</p>

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
				<label for="password" class="block text-sm font-medium text-gray-300 mb-1">
					Password <span class="text-gray-500 font-normal">(required)</span>
				</label>
				<input
					id="password"
					type="password"
					bind:value={password}
					required
					autocomplete="current-password"
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500/50"
					placeholder="••••••••"
				/>
			</div>

			{#if showTotp}
				<div class="bg-gray-900/30 border border-gray-700/50 rounded p-3 text-sm text-gray-300">
					This account has two-factor authentication enabled. Enter the 6-digit code from your authenticator app.
				</div>
				<div>
					<label for="totp" class="block text-sm font-medium text-gray-300 mb-1">Authenticator Code</label>
					<input
						id="totp"
						type="text"
						inputmode="numeric"
						maxlength="6"
						bind:value={totpCode}
						required
						autocomplete="one-time-code"
						class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500/50 tracking-widest text-center text-lg"
						placeholder="000000"
					/>
				</div>
			{/if}

			<button
				type="submit"
				disabled={loading || (showTotp && totpCode.length !== 6)}
				class="w-full py-2.5 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed text-white font-medium rounded transition-colors"
			>
				{loading ? 'Logging in…' : showTotp ? 'Verify Code' : 'Log In'}
			</button>
		</form>

		<!-- Secondary actions row -->
		<div class="flex items-center justify-between mt-4 text-sm">
			<a href="/register" class="text-gray-400 hover:text-white transition-colors">
				Don't have an account? <span class="underline">Register</span>
			</a>
			<a
				href="/login/forgot-password"
				onclick={(e) => { e.preventDefault(); showForgotPassword = !showForgotPassword; }}
				class="text-gray-400 hover:text-white transition-colors underline"
			>
				Forgot password?
			</a>
		</div>

		<!-- Divider -->
		<div class="flex items-center my-6">
			<div class="flex-1 border-t border-gray-700"></div>
			<span class="px-3 text-gray-500 text-xs">or continue with</span>
			<div class="flex-1 border-t border-gray-700"></div>
		</div>

		<!-- Steam Login -->
		<a
			href="/api/v1/auth/steam"
			class="w-full py-2.5 bg-gray-700 hover:bg-gray-600 text-white font-medium rounded transition-colors flex items-center justify-center gap-2"
			aria-label="Sign in with your Steam account"
		>
			<svg class="w-5 h-5" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
				<path
					d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm3.5-9c.83 0 1.5-.67 1.5-1.5S16.33 8 15.5 8 14 8.67 14 9.5s.67 1.5 1.5 1.5zm-7 0c.83 0 1.5-.67 1.5-1.5S9.33 8 8.5 8 7 8.67 7 9.5 7.67 11 8.5 11zm3.5 6.5c2.33 0 4.31-1.46 5.11-3.5H6.89c.8 2.04 2.78 3.5 5.11 3.5z"
				/>
			</svg>
			Sign in with Steam
		</a>

		{#if showForgotPassword}
			<form onsubmit={handleForgotSubmit} class="mt-6 space-y-4 border-t border-gray-700 pt-6">
				<h2 class="text-lg font-bold text-white">Reset Password</h2>
				<p class="text-gray-400 text-sm">Enter your email address and we'll send you a reset link.</p>

				{#if forgotError}
					<div role="alert" class="bg-red-900/30 border border-red-700/40 text-red-300 p-3 rounded text-sm">{forgotError}</div>
				{/if}

				{#if forgotSuccess}
					<div role="status" class="bg-green-900/30 border border-green-700/40 text-green-300 p-3 rounded text-sm">{forgotSuccess}</div>
				{/if}

				<div>
					<label for="forgot-email" class="block text-sm font-medium text-gray-300 mb-1">Email</label>
					<input
						id="forgot-email"
						type="email"
						bind:value={forgotEmail}
						required
						autocomplete="email"
						class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500/50"
						placeholder="you@example.com"
					/>
				</div>

				<button
					type="submit"
					disabled={forgotLoading}
					class="w-full py-2.5 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed text-white font-medium rounded transition-colors"
				>
					{forgotLoading ? 'Sending…' : 'Send Reset Link'}
				</button>

				<button
					type="button"
					onclick={() => (showForgotPassword = false)}
					class="w-full py-2 text-gray-400 hover:text-white transition-colors text-sm"
				>
					← Back to login
				</button>
			</form>
		{/if}
	</div>
</div>
