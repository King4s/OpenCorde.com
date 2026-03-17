<script lang="ts">
	/**
	 * @file Login page
	 * @purpose Email + password authentication
	 * @depends stores/auth
	 * @version 1.0.0
	 */
	import { goto } from '$app/navigation';
	import { login } from '$lib/stores/auth';

	let email = $state('');
	let password = $state('');
	let error = $state('');
	let loading = $state(false);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';
		loading = true;
		try {
			await login(email, password);
			window.location.href = '/servers';
		} catch (e: any) {
			error = e.message || 'Login failed';
		} finally {
			loading = false;
		}
	}
</script>

<div class="flex items-center justify-center min-h-screen bg-gray-900">
	<div class="w-full max-w-md p-8 bg-gray-800 rounded-lg shadow-xl">
		<h1 class="text-2xl font-bold text-white mb-2">Welcome back!</h1>
		<p class="text-gray-400 mb-6">Log in to OpenCorde</p>

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

			<button
				type="submit"
				disabled={loading}
				class="w-full py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white font-medium rounded transition-colors"
			>
				{loading ? 'Logging in...' : 'Log In'}
			</button>
		</form>

		<p class="text-gray-400 text-sm mt-4 text-center">
			Don't have an account? <a href="/register" class="text-indigo-400 hover:underline"
				>Register</a
			>
		</p>
	</div>
</div>
