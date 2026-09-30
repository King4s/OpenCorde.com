<script lang="ts">
	/**
	 * @file Instance Setup Wizard
	 * @purpose First-time setup for a fresh OpenCorde instance. Creates admin account and records instance metadata.
	 * @depends stores/auth
	 */
	import { goto } from '$app/navigation';
	import { browser } from '$app/environment';
	import { establishSession } from '$lib/stores/auth';
	import { onMount } from 'svelte';

	let step = $state(1);
	let instanceName = $state('');
	let baseUrl = $state('');
	let adminUsername = $state('');
	let adminEmail = $state('');
	let adminPassword = $state('');
	let confirmPassword = $state('');
	let error = $state('');
	let loading = $state(false);
	let checking = $state(true);

	onMount(async () => {
		if (!browser) return;
		try {
			const res = await fetch('/api/v1/setup/status');
			if (res.ok) {
				const data = await res.json();
				if (data.setup_completed) {
					window.location.replace('/');
					return;
				}
			}
			// Pre-fill base URL from current origin
			baseUrl = window.location.origin;
		} catch {
			// Allow setup to proceed even if status check fails
			baseUrl = window.location.origin;
		} finally {
			checking = false;
		}
	});

	function nextStep() {
		error = '';
		if (step === 1) {
			if (!instanceName.trim() || instanceName.length > 128) {
				error = 'Instance name must be between 1 and 128 characters.';
				return;
			}
			if (!baseUrl.trim() || baseUrl.length > 256) {
				error = 'Base URL must be between 1 and 256 characters.';
				return;
			}
			step = 2;
		}
	}

	function prevStep() {
		error = '';
		step = 1;
	}

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';
		loading = true;

		if (adminUsername.length < 2 || adminUsername.length > 32) {
			error = 'Username must be between 2 and 32 characters.';
			loading = false;
			return;
		}
		if (adminPassword.length < 8) {
			error = 'Password must be at least 8 characters.';
			loading = false;
			return;
		}
		if (adminPassword !== confirmPassword) {
			error = 'Passwords do not match.';
			loading = false;
			return;
		}

		try {
			const response = await fetch('/api/v1/setup/complete', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					instance_name: instanceName.trim(),
					base_url: baseUrl.trim(),
					admin_username: adminUsername.trim(),
					admin_email: adminEmail.trim(),
					admin_password: adminPassword
				})
			});

			if (!response.ok) {
				const data = await response.json().catch(() => ({}));
				throw new Error(data.message || `Setup failed (${response.status})`);
			}

			const result = await response.json();
			console.log('setup completed', result);

			// After setup, log the admin in automatically
			const loginRes = await fetch('/api/v1/auth/login', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					email: adminEmail.trim(),
					password: adminPassword
				})
			});

			if (loginRes.ok) {
				const loginData = await loginRes.json();
				const refreshToken = loginRes.headers.get('set-cookie')?.match(/refresh_token=([^;]+)/)?.[1];
				if (refreshToken) {
					localStorage.setItem('opencorde_refresh_token', refreshToken);
				}
				if (loginData.access_token) {
					await establishSession(loginData.access_token);
				}
			}

			window.location.replace('/servers');
		} catch (e: any) {
			error = e.message || 'Setup failed. Please try again.';
			loading = false;
		}
	}
</script>

{#if checking}
	<div class="flex items-center justify-center min-h-screen bg-gray-950">
		<div class="text-gray-400 text-sm">Checking instance status…</div>
	</div>
{:else}
	<div class="flex items-start justify-center min-h-screen bg-gray-950 px-4 py-6 sm:items-center sm:py-8">
		<div class="w-full max-w-lg p-6 sm:p-8 bg-gray-900 rounded-xl shadow-xl border border-gray-800">
			<div class="flex items-center gap-3 mb-6">
				<div class="w-10 h-10 rounded-lg bg-indigo-600/20 text-indigo-400 flex items-center justify-center border border-indigo-600/30">
					<svg class="w-5 h-5" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
						<path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
					</svg>
				</div>
				<div>
					<h1 class="text-xl font-bold text-white">Instance Setup</h1>
					<p class="text-gray-400 text-sm">Configure your OpenCorde server</p>
				</div>
			</div>

			<!-- Step indicator -->
			<div class="flex items-center gap-2 mb-6">
				<div class="flex-1 h-1.5 rounded-full {step >= 1 ? 'bg-indigo-600' : 'bg-gray-800'}"></div>
				<div class="flex-1 h-1.5 rounded-full {step >= 2 ? 'bg-indigo-600' : 'bg-gray-800'}"></div>
			</div>

			{#if error}
				<div role="alert" class="bg-red-900/30 border border-red-700/40 text-red-300 p-3 rounded mb-4 text-sm">{error}</div>
			{/if}

			{#if step === 1}
				<form onsubmit={(e) => { e.preventDefault(); nextStep(); }} class="space-y-4">
					<div>
						<label for="instance-name" class="block text-sm font-medium text-gray-300 mb-1">
							Instance Name <span class="text-gray-500 font-normal">(required)</span>
						</label>
						<input
							id="instance-name"
							type="text"
							bind:value={instanceName}
							required
							maxlength="128"
							class="w-full px-3 py-2 bg-gray-950 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500/50"
							placeholder="My OpenCorde Server"
						/>
						<p class="text-gray-500 text-xs mt-1">Shown to users and in invite links.</p>
					</div>

					<div>
						<label for="base-url" class="block text-sm font-medium text-gray-300 mb-1">
							Base URL <span class="text-gray-500 font-normal">(required)</span>
						</label>
						<input
							id="base-url"
							type="url"
							bind:value={baseUrl}
							required
							maxlength="256"
							class="w-full px-3 py-2 bg-gray-950 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500/50"
							placeholder="https://opencorde.example.com"
						/>
						<p class="text-gray-500 text-xs mt-1">The public URL of this instance.</p>
					</div>

					<button
						type="submit"
						class="w-full py-2.5 bg-indigo-600 hover:bg-indigo-500 text-white font-medium rounded transition-colors"
					>
						Continue →
					</button>
				</form>
			{:else if step === 2}
				<form onsubmit={handleSubmit} class="space-y-4">
					<div>
						<label for="admin-username" class="block text-sm font-medium text-gray-300 mb-1">
							Admin Username <span class="text-gray-500 font-normal">(required)</span>
						</label>
						<input
							id="admin-username"
							type="text"
							bind:value={adminUsername}
							required
							minlength="2"
							maxlength="32"
							autocomplete="username"
							class="w-full px-3 py-2 bg-gray-950 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500/50"
							placeholder="admin"
						/>
					</div>

					<div>
						<label for="admin-email" class="block text-sm font-medium text-gray-300 mb-1">
							Admin Email <span class="text-gray-500 font-normal">(required)</span>
						</label>
						<input
							id="admin-email"
							type="email"
							bind:value={adminEmail}
							required
							autocomplete="email"
							class="w-full px-3 py-2 bg-gray-950 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500/50"
							placeholder="admin@example.com"
						/>
					</div>

					<div>
						<label for="admin-password" class="block text-sm font-medium text-gray-300 mb-1">
							Password <span class="text-gray-500 font-normal">(min 8 characters)</span>
						</label>
						<input
							id="admin-password"
							type="password"
							bind:value={adminPassword}
							required
							minlength="8"
							autocomplete="new-password"
							class="w-full px-3 py-2 bg-gray-950 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500/50"
							placeholder="••••••••"
						/>
					</div>

					<div>
						<label for="confirm-password" class="block text-sm font-medium text-gray-300 mb-1">
							Confirm Password <span class="text-gray-500 font-normal">(required)</span>
						</label>
						<input
							id="confirm-password"
							type="password"
							bind:value={confirmPassword}
							required
							autocomplete="new-password"
							class="w-full px-3 py-2 bg-gray-950 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500/50"
							placeholder="••••••••"
						/>
					</div>

					<div class="flex gap-3 pt-2">
						<button
							type="button"
							onclick={prevStep}
							class="px-4 py-2.5 bg-gray-800 hover:bg-gray-700 text-gray-300 font-medium rounded transition-colors"
						>
							← Back
						</button>
						<button
							type="submit"
							disabled={loading}
							class="flex-1 py-2.5 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed text-white font-medium rounded transition-colors"
						>
							{loading ? 'Setting up…' : 'Complete Setup'}
						</button>
					</div>
				</form>
			{/if}
		</div>
	</div>
{/if}
