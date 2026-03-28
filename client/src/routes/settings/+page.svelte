<script lang="ts">
	/**
	 * @file User settings page
	 * @purpose Update username, email, upload avatar, and manage theme
	 */
	import { currentUser } from '$lib/stores/auth';
	import { themeStore } from '$lib/stores/theme';
	import { notificationsEnabled, registerPushToken, unregisterPushToken } from '$lib/stores/pushNotifications';
	import api from '$lib/api/client';
	import type { UserProfile } from '$lib/api/types';
	import TwoFactorSetup from '$lib/components/settings/TwoFactorSetup.svelte';

	const messageStyle = themeStore.messageStyle;

	let username = $state('');
	let email = $state('');
	let bio = $state('');
	let statusMessage = $state('');
	let saving = $state(false);
	let uploading = $state(false);
	let error = $state('');
	let success = $state('');
	let fileInput: HTMLInputElement;

	let totpEnabled = $state(false);

	// Sync form with store
	$effect(() => {
		if ($currentUser) {
			username = username || $currentUser.username;
			email = email || ($currentUser.email ?? '');
			bio = bio || ($currentUser.bio ?? '');
			statusMessage = statusMessage || ($currentUser.status_message ?? '');
			totpEnabled = $currentUser.totp_enabled;
		}
	});

	async function refreshProfile() {
		try {
			const profile = await api.get<UserProfile>('/users/@me');
			currentUser.set(profile);
		} catch (e) {
			// Silent fail, profile might be stale
		}
	}

	async function handleSave() {
		saving = true;
		error = '';
		success = '';
		try {
			const body: Record<string, string> = {};
			if (username.trim() !== $currentUser?.username) body.username = username.trim();
			if (email.trim() !== $currentUser?.email) body.email = email.trim();
			if (bio.trim() !== ($currentUser?.bio ?? '')) body.bio = bio.trim();
			if (statusMessage.trim() !== ($currentUser?.status_message ?? '')) body.status_message = statusMessage.trim();
			if (Object.keys(body).length === 0) { success = 'No changes to save.'; saving = false; return; }
			await api.patch('/users/@me', body);
			await refreshProfile();
			success = 'Settings saved.';
		} catch (e: any) {
			error = e.message ?? 'Failed to save';
		} finally {
			saving = false;
		}
	}

	async function handleAvatarUpload(e: Event) {
		const input = e.target as HTMLInputElement;
		if (!input.files?.[0]) return;
		uploading = true;
		error = '';
		success = '';
		try {
			const formData = new FormData();
			formData.append('file', input.files[0]);
			await api.postFormData<UserProfile>('/users/@me/avatar', formData);
			await refreshProfile();
			success = 'Avatar updated.';
		} catch (e: any) {
			error = e.message ?? 'Avatar upload failed';
		} finally {
			uploading = false;
			input.value = '';
		}
	}

	function getInitials(name: string) { return name.slice(0, 2).toUpperCase(); }
	function getColor(id: string) {
		const colors = ['bg-indigo-600','bg-purple-600','bg-pink-600','bg-red-600','bg-orange-600','bg-teal-600'];
		return colors[id.split('').reduce((a,c)=>a+c.charCodeAt(0),0)%colors.length];
	}

	// --- Push notifications ---
	let pushLoading = $state(false);
	let pushError = $state('');

	async function handlePushToggle() {
		pushLoading = true;
		pushError = '';
		try {
			if ($notificationsEnabled) {
				await unregisterPushToken();
			} else {
				await registerPushToken();
			}
		} catch (e: any) {
			pushError = e.message ?? 'Notification toggle failed';
		} finally {
			pushLoading = false;
		}
	}

	// --- Data export ---
	let exportLoading = $state(false);

	async function handleDataExport() {
		exportLoading = true;
		error = '';
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
			error = e.message ?? 'Export failed';
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

<div class="min-h-screen bg-gray-900 p-8">
	<div class="max-w-lg mx-auto">
		<div class="flex items-center gap-3 mb-6">
			<button onclick={() => history.back()} class="text-gray-400 hover:text-white text-sm">← Back</button>
			<h1 class="text-xl font-semibold text-white">User Settings</h1>
		</div>

		{#if error}
			<div class="mb-4 px-3 py-2 bg-red-900/40 border border-red-700/50 rounded text-red-300 text-sm">{error}</div>
		{/if}
		{#if success}
			<div class="mb-4 px-3 py-2 bg-green-900/40 border border-green-700/50 rounded text-green-300 text-sm">{success}</div>
		{/if}

		<!-- Avatar -->
		<div class="bg-gray-800 rounded-lg p-4 mb-4">
			<h2 class="text-sm font-semibold text-gray-400 uppercase mb-3">Avatar</h2>
			<div class="flex items-center gap-4">
				{#if $currentUser?.avatar_url}
					<img src={$currentUser.avatar_url} alt="avatar" class="w-16 h-16 rounded-full object-cover border-2 border-gray-700" />
				{:else if $currentUser}
					<div class="w-16 h-16 rounded-full {getColor($currentUser.id)} flex items-center justify-center text-white text-xl font-bold">
						{getInitials($currentUser.username)}
					</div>
				{/if}
				<button
					onclick={() => fileInput.click()}
					disabled={uploading}
					class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white text-sm rounded transition-colors"
				>
					{uploading ? 'Uploading…' : 'Change Avatar'}
				</button>
				<input type="file" bind:this={fileInput} accept="image/*" onchange={handleAvatarUpload} class="hidden" />
			</div>
		</div>

		<!-- Account info -->
		<div class="bg-gray-800 rounded-lg p-4 space-y-4 mb-4">
			<h2 class="text-sm font-semibold text-gray-400 uppercase">Account</h2>
			<div>
				<label class="block text-xs text-gray-400 mb-1" for="settings-username">Username</label>
				<input id="settings-username" type="text" bind:value={username} maxlength="32"
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-indigo-500" />
			</div>
			<div>
				<label class="block text-xs text-gray-400 mb-1" for="settings-email">Email</label>
				<input id="settings-email" type="email" bind:value={email}
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-indigo-500" />
			</div>
			<div>
				<label class="block text-xs text-gray-400 mb-1" for="settings-status">Status Message</label>
				<input id="settings-status" type="text" bind:value={statusMessage} maxlength="128" placeholder="What are you up to?"
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-indigo-500" />
			</div>
			<div>
				<label class="block text-xs text-gray-400 mb-1" for="settings-bio">Bio</label>
				<textarea id="settings-bio" bind:value={bio} maxlength="500" rows="3" placeholder="Tell others about yourself"
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-indigo-500 resize-none"></textarea>
			</div>
			<button onclick={handleSave} disabled={saving}
				class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white text-sm font-medium rounded transition-colors">
				{saving ? 'Saving…' : 'Save Changes'}
			</button>
		</div>

		<!-- Appearance -->
		<div class="bg-gray-800 rounded-lg p-4 space-y-4">
			<h2 class="text-sm font-semibold text-gray-400 uppercase">Appearance</h2>
			<div class="flex items-center justify-between">
				<span class="text-sm text-gray-300">Theme</span>
				<button onclick={() => themeStore.toggle()}
					class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white text-sm font-medium rounded transition-colors">
					{themeStore.isDark ? '☀️ Light Mode' : '🌙 Dark Mode'}
				</button>
			</div>

			<!-- Message Display -->
			<div>
				<span class="text-sm text-gray-300 block mb-2">Message Display</span>
				<div class="flex gap-2">
					<button
						onclick={() => $messageStyle === 'cozy' || themeStore.toggleMessageStyle()}
						class="px-4 py-2 text-sm font-medium rounded transition-colors {$messageStyle === 'cozy' ? 'bg-indigo-600 text-white' : 'bg-gray-700 text-gray-300 hover:bg-gray-600'}"
					>
						Cozy
					</button>
					<button
						onclick={() => $messageStyle === 'compact' || themeStore.toggleMessageStyle()}
						class="px-4 py-2 text-sm font-medium rounded transition-colors {$messageStyle === 'compact' ? 'bg-indigo-600 text-white' : 'bg-gray-700 text-gray-300 hover:bg-gray-600'}"
					>
						Compact
					</button>
				</div>
			</div>
		</div>

		<!-- Notifications -->
		<div class="bg-gray-800 rounded-lg p-4 space-y-4 mt-4">
			<h2 class="text-sm font-semibold text-gray-400 uppercase">Notifications</h2>
			{#if pushError}
				<div class="px-3 py-2 bg-red-900/40 border border-red-700/50 rounded text-red-300 text-sm">{pushError}</div>
			{/if}
			<div class="flex items-center justify-between">
				<div>
					<p class="text-sm text-gray-300">Push Notifications</p>
					<p class="text-xs text-gray-500 mt-0.5">Receive alerts for mentions even when the tab is in the background.</p>
				</div>
				<button
					onclick={handlePushToggle}
					disabled={pushLoading}
					class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors disabled:opacity-50
						{$notificationsEnabled ? 'bg-indigo-600' : 'bg-gray-600'}"
					aria-pressed={$notificationsEnabled}
					aria-label="Toggle push notifications"
				>
					<span class="inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform
						{$notificationsEnabled ? 'translate-x-6' : 'translate-x-1'}">
					</span>
				</button>
			</div>
		</div>

		<!-- Two-Factor Authentication -->
		<TwoFactorSetup enabled={totpEnabled} onchange={(val) => { totpEnabled = val; }} />

		<!-- Data & Privacy -->
		<div class="bg-gray-800 rounded-lg p-4 space-y-4 mt-4">
			<h2 class="text-sm font-semibold text-gray-400 uppercase">Data & Privacy</h2>
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
					<p class="text-sm text-red-400">Delete Account</p>
					<p class="text-xs text-gray-500 mt-0.5">Permanently delete your account and all personal data.</p>
				</div>
				<button
					onclick={() => { showDeleteModal = true; deletePassword = ''; deleteError = ''; }}
					class="px-3 py-1.5 bg-red-900/60 hover:bg-red-800 text-red-300 text-sm rounded transition-colors"
				>
					Delete Account
				</button>
			</div>
		</div>

		<!-- Admin Dashboard Link -->
		<div class="bg-gray-800 rounded-lg p-4 mt-4">
			<h2 class="text-sm font-semibold text-gray-400 uppercase mb-3">Administration</h2>
			<a href="/admin"
				class="inline-block px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white text-sm font-medium rounded transition-colors">
				Admin Dashboard
			</a>
		</div>
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
			<div class="px-3 py-2 bg-red-900/40 border border-red-700/50 rounded text-red-300 text-sm">{deleteError}</div>
		{/if}
		<div>
			<label class="block text-xs text-gray-400 mb-1" for="delete-password">Confirm your password</label>
			<input
				id="delete-password"
				type="password"
				bind:value={deletePassword}
				placeholder="Your current password"
				class="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-red-500"
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
				class="px-4 py-2 bg-red-700 hover:bg-red-600 disabled:opacity-50 text-white text-sm rounded transition-colors"
			>
				{deleting ? 'Deleting…' : 'Delete Forever'}
			</button>
		</div>
	</div>
</div>
{/if}
