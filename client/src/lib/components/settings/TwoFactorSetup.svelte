<script lang="ts">
	/**
	 * @file TwoFactorSetup — 2FA enable/disable panel for user settings
	 * @purpose Guides the user through TOTP setup (enable/verify) or disabling 2FA
	 * @depends api/client, api/types
	 */
	import api from '$lib/api/client';

	let { enabled = false, onchange }: { enabled: boolean; onchange: (enabled: boolean) => void } = $props();

	// --- Enable flow ---
	let step: 'idle' | 'setup' | 'verify' = $state('idle');
	let otpauthUrl = $state('');
	let secret = $state('');
	let verifyCode = $state('');
	let enableError = $state('');
	let enableLoading = $state(false);

	// --- Disable flow ---
	let showDisable = $state(false);
	let disableCode = $state('');
	let disableError = $state('');
	let disableLoading = $state(false);

	async function handleEnable() {
		enableLoading = true;
		enableError = '';
		try {
			const res = await api.post<{ otpauth_url: string; secret: string }>('/auth/2fa/enable');
			otpauthUrl = res.otpauth_url;
			secret = res.secret;
			step = 'setup';
		} catch (e: any) {
			enableError = e.message ?? 'Failed to start 2FA setup';
		} finally {
			enableLoading = false;
		}
	}

	async function handleVerify() {
		enableLoading = true;
		enableError = '';
		try {
			await api.post('/auth/2fa/verify', { code: verifyCode });
			step = 'idle';
			verifyCode = '';
			onchange(true);
		} catch (e: any) {
			enableError = e.code === 'UNAUTHORIZED' ? 'Invalid code — try again' : (e.message ?? 'Verification failed');
		} finally {
			enableLoading = false;
		}
	}

	async function handleDisable() {
		disableLoading = true;
		disableError = '';
		try {
			await api.delete('/auth/2fa', { code: disableCode });
			showDisable = false;
			disableCode = '';
			onchange(false);
		} catch (e: any) {
			disableError = e.code === 'UNAUTHORIZED' ? 'Invalid code — try again' : (e.message ?? 'Disable failed');
		} finally {
			disableLoading = false;
		}
	}

	function copySecret() {
		navigator.clipboard.writeText(secret).catch(() => {});
	}

	function cancelEnable() {
		step = 'idle';
		otpauthUrl = '';
		secret = '';
		verifyCode = '';
		enableError = '';
	}
</script>

<div class="bg-gray-800 rounded-lg p-4 space-y-3 mt-4">
	<div class="flex items-center justify-between">
		<div>
			<h2 class="text-sm font-semibold text-gray-400 uppercase">Two-Factor Authentication</h2>
			<p class="text-xs text-gray-500 mt-0.5">Protect your account with a TOTP authenticator app.</p>
		</div>
		<span class="text-xs font-medium px-2 py-1 rounded-full {enabled ? 'bg-green-900/60 text-green-300' : 'bg-gray-700 text-gray-400'}">
			{enabled ? 'Enabled' : 'Disabled'}
		</span>
	</div>

	{#if !enabled && step === 'idle'}
		{#if enableError}
			<div class="px-3 py-2 bg-red-900/40 border border-red-700/50 rounded text-red-300 text-sm">{enableError}</div>
		{/if}
		<button
			onclick={handleEnable}
			disabled={enableLoading}
			class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white text-sm font-medium rounded transition-colors"
		>
			{enableLoading ? 'Generating…' : 'Enable 2FA'}
		</button>

	{:else if step === 'setup'}
		<div class="space-y-3">
			<p class="text-sm text-gray-300">
				Scan the code below in your authenticator app (Google Authenticator, Authy, etc.) or enter the secret manually.
			</p>
			<div class="bg-gray-900 rounded p-3 space-y-2">
				<p class="text-xs text-gray-500 uppercase font-medium">Manual entry secret</p>
				<div class="flex items-center gap-2">
					<code class="flex-1 text-green-400 text-sm font-mono break-all">{secret}</code>
					<button
						onclick={copySecret}
						class="px-2 py-1 bg-gray-700 hover:bg-gray-600 text-gray-300 text-xs rounded transition-colors whitespace-nowrap"
					>
						Copy
					</button>
				</div>
				<a
					href={otpauthUrl}
					class="block text-xs text-indigo-400 hover:underline break-all"
					title="Open in authenticator app"
				>
					Open otpauth:// link
				</a>
			</div>

			{#if enableError}
				<div class="px-3 py-2 bg-red-900/40 border border-red-700/50 rounded text-red-300 text-sm">{enableError}</div>
			{/if}

			<div>
				<label for="2fa-verify-code" class="block text-xs text-gray-400 mb-1">Enter the 6-digit code to confirm setup</label>
				<input
					id="2fa-verify-code"
					type="text"
					inputmode="numeric"
					maxlength="6"
					bind:value={verifyCode}
					autocomplete="one-time-code"
					placeholder="000000"
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm tracking-widest text-center focus:outline-none focus:border-indigo-500"
				/>
			</div>

			<div class="flex gap-2">
				<button
					onclick={handleVerify}
					disabled={enableLoading || verifyCode.length !== 6}
					class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white text-sm font-medium rounded transition-colors"
				>
					{enableLoading ? 'Verifying…' : 'Activate 2FA'}
				</button>
				<button
					onclick={cancelEnable}
					class="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-gray-300 text-sm rounded transition-colors"
				>
					Cancel
				</button>
			</div>
		</div>

	{:else if enabled}
		{#if !showDisable}
			<button
				onclick={() => { showDisable = true; disableCode = ''; disableError = ''; }}
				class="px-4 py-2 bg-red-900/60 hover:bg-red-800 text-red-300 text-sm rounded transition-colors"
			>
				Disable 2FA
			</button>
		{:else}
			<div class="space-y-3">
				{#if disableError}
					<div class="px-3 py-2 bg-red-900/40 border border-red-700/50 rounded text-red-300 text-sm">{disableError}</div>
				{/if}
				<div>
					<label for="2fa-disable-code" class="block text-xs text-gray-400 mb-1">Enter your current authenticator code to confirm</label>
					<input
						id="2fa-disable-code"
						type="text"
						inputmode="numeric"
						maxlength="6"
						bind:value={disableCode}
						autocomplete="one-time-code"
						placeholder="000000"
						class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm tracking-widest text-center focus:outline-none focus:border-indigo-500"
					/>
				</div>
				<div class="flex gap-2">
					<button
						onclick={handleDisable}
						disabled={disableLoading || disableCode.length !== 6}
						class="px-4 py-2 bg-red-700 hover:bg-red-600 disabled:opacity-50 text-white text-sm font-medium rounded transition-colors"
					>
						{disableLoading ? 'Disabling…' : 'Confirm Disable'}
					</button>
					<button
						onclick={() => showDisable = false}
						class="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-gray-300 text-sm rounded transition-colors"
					>
						Cancel
					</button>
				</div>
			</div>
		{/if}
	{/if}
</div>
