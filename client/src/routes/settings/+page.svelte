<script lang="ts">
	/**
	 * @file User settings hub — identity, security, appearance, notifications, and privacy
	 * @purpose Discoverable tabbed shell that ties account and identity surfaces together
	 * @depends $lib/components/settings/*
	 */
	import ProfileTab from '$lib/components/settings/ProfileTab.svelte';
	import SecurityTab from '$lib/components/settings/SecurityTab.svelte';
	import AppearanceTab from '$lib/components/settings/AppearanceTab.svelte';
	import NotificationsTab from '$lib/components/settings/NotificationsTab.svelte';
	import PrivacyTab from '$lib/components/settings/PrivacyTab.svelte';

	type TabId = 'profile' | 'security' | 'appearance' | 'notifications' | 'privacy' | 'admin';

	interface TabDef {
		id: TabId;
		label: string;
	}

	const tabs: TabDef[] = [
		{ id: 'profile',       label: 'Profile' },
		{ id: 'security',      label: 'Security' },
		{ id: 'appearance',    label: 'Appearance' },
		{ id: 'notifications', label: 'Notifications' },
		{ id: 'privacy',       label: 'Privacy' },
		{ id: 'admin',         label: 'Administration' },
	];

	let activeTab = $state<TabId>('profile');
</script>

<div class="min-h-screen bg-gray-900 px-3 py-4 sm:px-4 sm:py-6 lg:px-8">
	<div class="max-w-5xl mx-auto grid grid-cols-1 lg:grid-cols-[240px_1fr] gap-4 sm:gap-6">
		<!-- Sidebar -->
		<aside class="lg:sticky lg:top-6 h-fit bg-gray-800 rounded-xl border border-gray-700 p-3">
			<div class="flex items-center justify-between mb-4">
				<h1 class="text-lg font-semibold text-white">Settings</h1>
				<button onclick={() => history.back()} class="text-gray-400 hover:text-white text-sm">Back</button>
			</div>
			<nav class="space-y-1 text-sm">
				{#each tabs as tab}
					<button
						onclick={() => activeTab = tab.id}
						class="block w-full text-left px-3 py-2 rounded transition-colors
							{activeTab === tab.id ? 'bg-gray-700 text-white' : 'text-gray-300 hover:bg-gray-700'}"
					>
						{tab.label}
					</button>
				{/each}
			</nav>
		</aside>

		<!-- Content -->
		<section class="space-y-4">
			{#if activeTab === 'profile'}
				<ProfileTab />
			{:else if activeTab === 'security'}
				<SecurityTab />
			{:else if activeTab === 'appearance'}
				<AppearanceTab />
			{:else if activeTab === 'notifications'}
				<NotificationsTab />
			{:else if activeTab === 'privacy'}
				<PrivacyTab />
			{:else if activeTab === 'admin'}
				<div class="bg-gray-800 rounded-lg p-3 sm:p-4">
					<h2 class="text-sm font-semibold text-gray-400 uppercase mb-3">Administration</h2>
					<a href="/admin"
						class="inline-block px-4 py-2 bg-gray-600 hover:bg-gray-700 text-white text-sm font-medium rounded transition-colors">
						Admin Dashboard
					</a>
				</div>
			{/if}
		</section>
	</div>
</div>
