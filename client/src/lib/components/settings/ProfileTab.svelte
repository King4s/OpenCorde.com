<script lang="ts">
	/**
	 * @file ProfileTab — user profile editing with avatar, name, email, status, and bio
	 * @purpose Self-contained settings panel for editing core profile fields
	 * @depends $lib/stores/auth, $lib/api/client, $lib/api/types
	 */
	import { currentUser } from '$lib/stores/auth';
	import api from '$lib/api/client';
	import type { UserProfile } from '$lib/api/types';
	import StatusPicker from '$lib/components/user/StatusPicker.svelte';
	import AccountSwitcher from '$lib/components/settings/AccountSwitcher.svelte';
	import ActivityStatusPicker from '$lib/components/settings/ActivityStatusPicker.svelte';
	import UserBadges from '$lib/components/settings/UserBadges.svelte';

	let username = $state('');
	let email = $state('');
	let bio = $state('');
	let statusMessage = $state('');
	let saving = $state(false);
	let uploading = $state(false);
	let error = $state('');
	let success = $state('');
	let fileInput: HTMLInputElement;

	let showStatusPicker = $state(false);

	$effect(() => {
		if ($currentUser) {
			username = username || $currentUser.username;
			email = email || ($currentUser.email ?? '');
			bio = bio || ($currentUser.bio ?? '');
			statusMessage = statusMessage || ($currentUser.status_message ?? '');
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
		const colors = ['bg-gray-600','bg-gray-600','bg-gray-600','bg-gray-600','bg-gray-600','bg-gray-600'];
		return colors[id.split('').reduce((a,c)=>a+c.charCodeAt(0),0)%colors.length];
	}

	function statusLabel(status: number): string {
		const map: Record<number, string> = { 0: 'Online', 1: 'Idle', 2: 'Do Not Disturb', 3: 'Invisible' };
		return map[status] ?? 'Online';
	}

	function statusColor(status: number): string {
		const map: Record<number, string> = { 0: '#23a55a', 1: '#f0b232', 2: '#f23f43', 3: '#747f8d' };
		return map[status] ?? '#23a55a';
	}
</script>

{#if error}
	<div class="mb-3 px-3 py-2 bg-gray-900/40 border border-gray-700/50 rounded text-gray-300 text-sm">{error}</div>
{/if}
{#if success}
	<div class="mb-3 px-3 py-2 bg-gray-900/40 border border-gray-700/50 rounded text-gray-300 text-sm">{success}</div>
{/if}

<!-- Avatar -->
<div id="profile" class="bg-gray-800 rounded-lg p-3 sm:p-4 mb-4">
	<h2 class="text-sm font-semibold text-gray-400 uppercase mb-3">Profile</h2>
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
			class="px-3 py-1.5 bg-gray-600 hover:bg-gray-700 disabled:opacity-50 text-white text-sm rounded transition-colors"
		>
			{uploading ? 'Uploading…' : 'Change Avatar'}
		</button>
		<input type="file" bind:this={fileInput} accept="image/*" onchange={handleAvatarUpload} class="hidden" />
	</div>
</div>

<!-- Account info -->
<div id="account" class="bg-gray-800 rounded-lg p-3 sm:p-4 mb-4">
	<h2 class="text-sm font-semibold text-gray-400 uppercase">Account</h2>
	<div>
		<label class="block text-xs text-gray-400 mb-1" for="settings-username">Username</label>
		<input id="settings-username" type="text" bind:value={username} maxlength="32"
			class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-gray-500" />
	</div>
	<div>
		<label class="block text-xs text-gray-400 mb-1" for="settings-email">Email</label>
		<input id="settings-email" type="email" bind:value={email}
			class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-gray-500" />
	</div>
	<div>
		<label class="block text-xs text-gray-400 mb-1" for="settings-status">Status Message</label>
		<input id="settings-status" type="text" bind:value={statusMessage} maxlength="128" placeholder="What are you up to?"
			class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-gray-500" />
	</div>
	<div>
		<label class="block text-xs text-gray-400 mb-1" for="settings-bio">Bio</label>
		<textarea id="settings-bio" bind:value={bio} maxlength="500" rows="3" placeholder="Tell others about yourself"
			class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-gray-500 resize-none"></textarea>
	</div>
	<button onclick={handleSave} disabled={saving}
		class="px-4 py-2 bg-gray-600 hover:bg-gray-700 disabled:opacity-50 text-white text-sm font-medium rounded transition-colors">
		{saving ? 'Saving…' : 'Save Changes'}
	</button>
</div>

<!-- Status display & picker -->
<div class="bg-gray-800 rounded-lg p-3 sm:p-4 mb-4">
	<h2 class="text-sm font-semibold text-gray-400 uppercase mb-3">Online Status</h2>
	<div class="flex items-center gap-3 mb-2">
		<span class="inline-block w-3 h-3 rounded-full flex-shrink-0" style="background-color: {statusColor($currentUser?.status ?? 0)}"></span>
		<span class="text-sm text-gray-300">{statusLabel($currentUser?.status ?? 0)}</span>
	</div>
	<button
		onclick={() => { showStatusPicker = !showStatusPicker; }}
		class="px-3 py-1.5 bg-gray-600 hover:bg-gray-700 text-white text-sm rounded transition-colors"
	>
		{showStatusPicker ? 'Close Status Picker' : 'Set Status'}
	</button>

	{#if showStatusPicker}
		<div class="mt-3 bg-gray-900 rounded-lg border border-gray-700 p-0">
			<!-- Inline wrapper for StatusPicker popover — we use a simplified inline version -->
			<div class="p-3 text-xs text-gray-500">
				Status picker is available from the user panel. Use the status icon in the bottom-left corner of the app.
			</div>
		</div>
	{/if}
</div>

<!-- Activity status (stub) -->
<ActivityStatusPicker />

<!-- Badges (stub) -->
<UserBadges />

<!-- Account switcher (stub) -->
<AccountSwitcher />
