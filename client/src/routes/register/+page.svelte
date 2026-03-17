<script lang="ts">
	/**
	 * @file Register page
	 * @purpose User registration with email, username, and password
	 * @depends stores/auth
	 * @version 1.0.0
	 */
	import { goto } from '$app/navigation';
	import { register } from '$lib/stores/auth';

	let email = $state('');
	let username = $state('');
	let password = $state('');
	let passwordConfirm = $state('');
	let error = $state('');
	let loading = $state(false);

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
	</div>
</div>
