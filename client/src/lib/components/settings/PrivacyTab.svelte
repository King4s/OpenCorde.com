<script lang="ts">
	/**
	 * @file PrivacyTab — data export and account deletion
	 * @purpose Download personal data and permanently delete account
	 * @depends api/client
	 */
	import api from '$lib/api/client';

	// --- Data export ---
	let exportLoading = $state(false);
	let exportError = $state('');

	async function handleDataExport() {
		exportLoading = true;
		exportError = '';
		try {
			const token = localStorage.getItem('opencorde_token');
			const res = await fetch('/api/v1/users/@me/export', {
				headers: { Authorization: `Bearer ${token}` }
			});
			if (!res.ok) throw new Error(await res.text());
			const blob = await res.blob();
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = 'opencorde-data-export.json';
			a.click();
			URL.revokeObjectURL(url);
		} catch (e: any) {
			exportError = e.message ?? 'Export failed';
		} finally {
			exportLoading = false;
		}
	}

	// --- Account deletion ---
	let showDeleteModal = $state(false);
	let deletePassword = $state('');
	let deleteError = $state('');
	let deleting = $state(false);

	async function handleDeleteAccount() {
		deleting = true;
		deleteError = '';
		try {
			await api.delete('/users/@me', { password: deletePassword });
			localStorage.removeItem('opencorde_token');
			window.location.href = '/';
		} catch (e: any) {
			deleteError = e.message ?? 'Deletion failed';
		} finally {
			deleting = false;
		}
	}
</script>

<div class="bg-gray-800 rounded-lg p-4 space-y-4">
	<h2 class="text-sm font-semibold text-gray-400 uppercase">Data &amp; Privacy</h2>

	{#if exportError}
		<div class="px-3 py-2 bg-gray-900/40 border border-gray-700/50 rounded text-gray-300 text-sm">{exportError}</div>
	{/if}

	<div class="flex items-center justify-between">
		<div>
			<p class="text-sm text-gray-300">Download Your Data</p>
			<p class="text-xs text-gray-500 mt-0.5">Get a JSON file of your profile, messages, and files.</p>
		</div>
		<button
			onclick={handleDataExport}
			disabled={exportLoading}
			class="px-3 py-1.5 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 text-white text-sm rounded transition-colors"
		>
			{exportLoading ? 'Exporting…' : 'Export Data'}
		</button>
	</div>
	<hr class="border-gray-700" />
	<div class="flex items-center justify-between">
		<div>
			<p class="text-sm text-gray-400">Delete Account</p>
			<p class="text-xs text-gray-500 mt-0.5">Permanently delete your account and all personal data.</p>
		</div>
		<button
			onclick={() => { showDeleteModal = true; deletePassword = ''; deleteError = ''; }}
			class="px-3 py-1.5 bg-gray-900/60 hover:bg-gray-800 text-gray-300 text-sm rounded transition-colors"
		>
			Delete Account
		</button>
	</div>
</div>

<!-- Delete Account Modal -->
{#if showDeleteModal}
<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
	<div class="bg-gray-900 border border-gray-700 rounded-xl p-6 w-full max-w-sm mx-4 space-y-4">
		<h2 class="text-lg font-semibold text-white">Delete Account</h2>
		<p class="text-sm text-gray-400">
			This will permanently delete your account, messages, files, and all personal data. This cannot be undone.
		</p>
		{#if deleteError}
			<div class="px-3 py-2 bg-gray-900/40 border border-gray-700/50 rounded text-gray-300 text-sm">{deleteError}</div>
		{/if}
		<div>
			<label class="block text-xs text-gray-400 mb-1" for="delete-password">Confirm your password</label>
			<input
				id="delete-password"
				type="password"
				bind:value={deletePassword}
				placeholder="Your current password"
				class="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-gray-500"
			/>
		</div>
		<div class="flex gap-3 justify-end">
			<button
				onclick={() => showDeleteModal = false}
				class="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white text-sm rounded transition-colors"
			>
				Cancel
			</button>
			<button
				onclick={handleDeleteAccount}
				disabled={deleting || deletePassword.length === 0}
				class="px-4 py-2 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 text-white text-sm rounded transition-colors"
			>
				{deleting ? 'Deleting…' : 'Delete Forever'}
			</button>
		</div>
	</div>
</div>
{/if}
