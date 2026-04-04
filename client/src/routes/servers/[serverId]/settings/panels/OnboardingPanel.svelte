<script lang="ts">
	/**
	 * @file OnboardingPanel — Space onboarding configuration
	 * @purpose Allow server owners to set up welcome message and prompts shown to new members
	 * @section settings/onboarding
	 */
	import { onMount } from 'svelte';
	import api from '$lib/api/client';

	interface Props {
		spaceId: string;
	}

	let { spaceId }: Props = $props();

	let enabled = $state(false);
	let welcomeMessage = $state('');
	let loading = $state(true);
	let saving = $state(false);
	let saved = $state(false);
	let error = $state('');

	onMount(async () => {
		try {
			const d = await api.get<{ enabled: boolean; welcome_message: string | null }>(`/servers/${spaceId}/onboarding`);
			enabled = d.enabled;
			welcomeMessage = d.welcome_message ?? '';
		} catch {
			// No onboarding config yet — defaults are fine
		} finally {
			loading = false;
		}
	});

	async function handleSave() {
		saving = true;
		error = '';
		saved = false;
		try {
			await api.put(`/servers/${spaceId}/onboarding`, {
				enabled,
				welcome_message: welcomeMessage.trim() || null,
				prompts: [],
			});
			saved = true;
			setTimeout(() => { saved = false; }, 2500);
		} catch (e: any) {
			error = e.message ?? 'Failed to save';
		} finally {
			saving = false;
		}
	}
</script>

<div class="p-8 max-w-2xl">
	<h2 class="text-xl font-bold text-white mb-1">Space Onboarding</h2>
	<p class="text-gray-400 text-sm mb-6">
		Show a welcome screen to new members when they first join your server.
	</p>

	{#if loading}
		<p class="text-gray-500 text-sm">Loading...</p>
	{:else}
		<div class="space-y-6">
			<!-- Enable toggle -->
			<label class="flex items-center gap-3 cursor-pointer">
				<div class="relative">
					<input type="checkbox" bind:checked={enabled} class="sr-only" />
					<div class="w-10 h-5 rounded-full transition-colors {enabled ? 'bg-gray-600' : 'bg-gray-600'}">
						<div class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {enabled ? 'translate-x-5' : 'translate-x-0'}"></div>
					</div>
				</div>
				<span class="text-sm font-medium text-gray-200">Enable onboarding for new members</span>
			</label>

			{#if enabled}
				<!-- Welcome message -->
				<div>
					<label for="onboarding-welcome" class="block text-sm font-medium text-gray-300 mb-1">
						Welcome message
					</label>
					<textarea
						id="onboarding-welcome"
						bind:value={welcomeMessage}
						rows="4"
						maxlength="1000"
						placeholder="Welcome to the server! Here's what you should know..."
						class="w-full px-3 py-2 bg-gray-800 border border-gray-600 rounded-lg text-white text-sm placeholder-gray-500 resize-y outline-none focus:border-gray-500"
					></textarea>
					<p class="text-gray-500 text-xs mt-1">{welcomeMessage.length}/1000 characters</p>
				</div>

				<div class="bg-gray-800/50 border border-gray-700 rounded-lg p-4">
					<p class="text-gray-400 text-sm">
						<span class="text-gray-300 font-medium">Prompts</span> — Custom role/channel selection prompts
						for new members are not yet available. They will appear here in a future update.
					</p>
				</div>
			{/if}

			{#if error}
				<p class="text-gray-400 text-sm">{error}</p>
			{/if}

			<div class="flex items-center gap-3">
				<button
					onclick={handleSave}
					disabled={saving}
					class="px-5 py-2 bg-gray-600 hover:bg-gray-700 disabled:opacity-50 text-white text-sm font-medium rounded-lg transition-colors"
				>{saving ? 'Saving...' : 'Save Changes'}</button>
				{#if saved}
					<span class="text-gray-400 text-sm">Saved!</span>
				{/if}
			</div>
		</div>
	{/if}
</div>
